use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    value: String,
    cursor: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    draft: Option<String>,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self {
            value,
            cursor,
            history: Vec::new(),
            history_index: None,
            draft: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.chars().count();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.history_index = None;
        self.draft = None;
    }

    pub fn submit(&mut self) {
        let trimmed = self.value.trim().to_string();
        if !trimmed.is_empty() {
            if self.history.last() != Some(&trimmed) {
                self.history.push(trimmed);
            }
        }
        self.value.clear();
        self.cursor = 0;
        self.history_index = None;
        self.draft = None;
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.history_index.is_none() {
            self.draft = Some(self.value.clone());
            self.history_index = Some(self.history.len().saturating_sub(1));
        } else {
            let index = self.history_index.unwrap();
            if index > 0 {
                self.history_index = Some(index - 1);
            } else {
                return;
            }
        }
        if let Some(index) = self.history_index {
            self.value = self.history[index].clone();
            self.cursor = self.value.chars().count();
        }
    }

    pub fn history_down(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 < self.history.len() {
            self.history_index = Some(index + 1);
            self.value = self.history[index + 1].clone();
            self.cursor = self.value.chars().count();
        } else {
            self.history_index = None;
            self.value = self.draft.take().unwrap_or_default();
            self.cursor = self.value.chars().count();
        }
    }

    pub fn cursor_offset(&self) -> usize {
        self.width_until_cursor()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert(character);
                true
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Delete => {
                self.delete();
                true
            }
            KeyCode::Left => {
                self.move_left();
                true
            }
            KeyCode::Right => {
                self.move_right();
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.value.chars().count();
                true
            }
            KeyCode::Up => {
                self.history_up();
                true
            }
            KeyCode::Down => {
                self.history_down();
                true
            }
            _ => false,
        }
    }

    fn insert(&mut self, character: char) {
        let byte_index = self.byte_index(self.cursor);
        self.value.insert(byte_index, character);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        if self.cursor == self.value.chars().count() {
            return;
        }

        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.value.chars().count());
    }

    fn byte_index(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(index, _)| index)
            .unwrap_or_else(|| self.value.len())
    }

    fn width_until_cursor(&self) -> usize {
        let byte_index = self.byte_index(self.cursor);
        UnicodeWidthStr::width(&self.value[..byte_index])
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::TextInput;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn edits_text_and_moves_cursor() {
        let mut input = TextInput::new("manual");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Char('X')));
        input.handle_key(key(KeyCode::Backspace));
        input.handle_key(key(KeyCode::Delete));

        assert_eq!(input.value(), "manul");
        assert_eq!(input.cursor_offset(), 4);
    }

    #[test]
    fn handles_wide_characters_for_cursor_width() {
        let mut input = TextInput::new("华工");
        input.handle_key(key(KeyCode::Left));

        assert_eq!(input.cursor_offset(), 2);
    }

    #[test]
    fn records_history_on_submit() {
        let mut input = TextInput::default();
        input.set_value("manual");
        input.submit();
        assert_eq!(input.value(), "");
        assert_eq!(input.history, vec!["manual"]);
    }

    #[test]
    fn skips_empty_commands_in_history() {
        let mut input = TextInput::default();
        input.set_value("   ");
        input.submit();
        assert!(input.history.is_empty());
    }

    #[test]
    fn deduplicates_consecutive_history() {
        let mut input = TextInput::default();
        input.set_value("help");
        input.submit();
        input.set_value("help");
        input.submit();
        assert_eq!(input.history, vec!["help"]);
    }

    #[test]
    fn navigates_history_with_up_and_down() {
        let mut input = TextInput::default();
        input.set_value("first");
        input.submit();
        input.set_value("second");
        input.submit();

        // Press Up twice
        input.history_up();
        assert_eq!(input.value(), "second");
        input.history_up();
        assert_eq!(input.value(), "first");

        // Press Down twice
        input.history_down();
        assert_eq!(input.value(), "second");
        input.history_down();
        assert_eq!(input.value(), "");
    }

    #[test]
    fn preserves_draft_when_browsing_history() {
        let mut input = TextInput::default();
        input.set_value("history-cmd");
        input.submit();

        input.set_value("draft text");
        input.history_up();
        assert_eq!(input.value(), "history-cmd");

        input.history_down();
        assert_eq!(input.value(), "draft text");
    }
}
