use std::{fs, path::PathBuf, time::Duration};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    content::{Block, DocumentContent, RichContentRenderCache},
    input::TextInput,
    manual::{self, Entry, ManualRepo, Section},
    ui,
};

pub type AppTerminal = Terminal<CrosstermBackend<std::io::Stdout>>;

pub const DEFAULT_MANUAL_REPO: &str = "~/survive-in-scut";
pub const MIN_TERMINAL_WIDTH: u16 = 100;
pub const MIN_TERMINAL_HEIGHT: u16 = 24;

const TICK_RATE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    PathPrompt,
    Home,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualFocus {
    Sections,
    Entries,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMessage {
    pub kind: StatusKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub title: String,
    pub relative_path: PathBuf,
    source: String,
    render_cache: Option<RichContentRenderCache>,
}

#[derive(Debug)]
pub struct ManualState {
    pub repo_path: PathBuf,
    pub repo: Option<ManualRepo>,
    pub error: Option<String>,
    pub focus: ManualFocus,
    pub section_cursor: usize,
    pub active_section: usize,
    pub entry_cursor: usize,
    pub active_entry: Option<usize>,
    pub content_scroll: u16,
    pub content_viewport_height: u16,
    pub content_dual_column: bool,
    pub loaded_document: Option<LoadedDocument>,
}

#[derive(Debug)]
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub manual_repo_path: PathBuf,
    pub command_input: TextInput,
    pub path_input: TextInput,
    pub status: Option<StatusMessage>,
    pub manual_state: Option<ManualState>,
}

impl App {
    pub fn new() -> Self {
        let default_path = PathBuf::from(DEFAULT_MANUAL_REPO);
        let default_input = default_path.display().to_string();
        let path_valid = manual::validate_repo_root(&default_path).is_ok();

        Self {
            mode: AppMode::Home,
            should_quit: false,
            manual_repo_path: default_path,
            command_input: TextInput::default(),
            path_input: TextInput::new(default_input),
            status: Some(if path_valid {
                StatusMessage {
                    kind: StatusKind::Info,
                    text: "输入 `manual` 进入手册浏览模式。".to_string(),
                }
            } else {
                StatusMessage {
                    kind: StatusKind::Error,
                    text: "默认手册仓库无效，输入 `manual` 时需指定一个包含 docs/ 的本地路径。".to_string(),
                }
            }),
            manual_state: None,
        }
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if matches!(key.code, KeyCode::Char('q')) {
            self.should_quit = true;
            return;
        }

        match self.mode {
            AppMode::PathPrompt => self.handle_path_prompt(key),
            AppMode::Home => self.handle_home(key),
            AppMode::Manual => self.handle_manual(key),
        }
    }

    fn handle_path_prompt(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Enter) {
            let candidate = PathBuf::from(self.path_input.value().trim());
            if manual::validate_repo_root(&candidate).is_ok() {
                self.manual_repo_path = candidate;
                self.mode = AppMode::Home;
                self.status = Some(StatusMessage {
                    kind: StatusKind::Info,
                    text: "路径验证成功，输入 `manual` 进入手册浏览模式。".to_string(),
                });
            } else {
                self.status = Some(StatusMessage {
                    kind: StatusKind::Error,
                    text: "路径无效：必须是存在的目录，并且包含 docs/ 子目录。".to_string(),
                });
            }
            return;
        }

        self.path_input.handle_key(key);
    }

    fn handle_home(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Enter) {
            let command = self.command_input.value().trim();
            match command {
                "manual" => {
                    self.open_manual_mode();
                }
                "help" => {
                    self.status = Some(StatusMessage {
                        kind: StatusKind::Info,
                        text: "可用命令：manual（浏览手册）、help（显示帮助）、exit（退出程序）。".to_string(),
                    });
                }
                "exit" => {
                    self.should_quit = true;
                }
                "" => {}
                other => {
                    self.status = Some(StatusMessage {
                        kind: StatusKind::Error,
                        text: format!("未知命令：`{other}`。输入 `help` 查看可用命令。"),
                    });
                }
            }
            self.command_input.clear();
            return;
        }

        self.command_input.handle_key(key);
    }

    fn handle_manual(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Esc) {
            self.mode = AppMode::Home;
            self.status = Some(StatusMessage {
                kind: StatusKind::Info,
                text: "已返回首页。输入 `manual` 可重新进入手册浏览模式。".to_string(),
            });
            return;
        }

        let Some(manual_state) = self.manual_state.as_mut() else {
            return;
        };

        match key.code {
            KeyCode::Left => manual_state.focus_left(),
            KeyCode::Right => manual_state.focus_right(),
            KeyCode::Up => manual_state.move_up(),
            KeyCode::Down => manual_state.move_down(),
            KeyCode::Enter => manual_state.confirm_focus(),
            KeyCode::Char('t') => manual_state.toggle_dual_column(),
            _ => {}
        }
    }

    fn open_manual_mode(&mut self) {
        if manual::validate_repo_root(&self.manual_repo_path).is_err() {
            self.mode = AppMode::PathPrompt;
            self.status = Some(StatusMessage {
                kind: StatusKind::Error,
                text: "当前手册仓库路径无效，请输入一个包含 docs/ 的本地路径。".to_string(),
            });
            return;
        }
        self.manual_state = Some(ManualState::new(self.manual_repo_path.clone()));
        self.mode = AppMode::Manual;
        self.status = None;
    }
}

impl LoadedDocument {
    fn from_entry(entry: &Entry) -> Self {
        let source = match fs::read_to_string(&entry.source_path) {
            Ok(source) => source,
            Err(error) => format!(
                "读取文档失败：{} ({error})",
                entry.source_path.display()
            ),
        };

        Self {
            title: entry.title.clone(),
            relative_path: entry.relative_path.clone(),
            source,
            render_cache: None,
        }
    }

    fn ensure_cache(&mut self, width: usize) -> &RichContentRenderCache {
        let width = width.max(12);
        if self.render_cache.as_ref().map(|cache| cache.width) != Some(width) {
            self.render_cache = Some(RichContentRenderCache::new(&self.source, width));
        }

        self.render_cache
            .as_ref()
            .expect("render cache should exist after ensure_cache")
    }
}

impl ManualState {
    pub fn new(repo_path: PathBuf) -> Self {
        let mut state = Self {
            repo_path,
            repo: None,
            error: None,
            focus: ManualFocus::Sections,
            section_cursor: 0,
            active_section: 0,
            entry_cursor: 0,
            active_entry: None,
            content_scroll: 0,
            content_viewport_height: 0,
            content_dual_column: false,
            loaded_document: None,
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        self.focus = ManualFocus::Sections;
        self.section_cursor = 0;
        self.active_section = 0;
        self.entry_cursor = 0;
        self.active_entry = None;
        self.content_scroll = 0;
        self.content_viewport_height = 0;
        self.loaded_document = None;

        match ManualRepo::load(&self.repo_path) {
            Ok(repo) => {
                self.error = None;
                self.repo = Some(repo);
                if !self.sections().is_empty() {
                    self.activate_section(0);
                }
            }
            Err(error) => {
                self.repo = None;
                self.error = Some(error);
            }
        }
    }

    pub fn sections(&self) -> &[Section] {
        self.repo
            .as_ref()
            .map(|repo| repo.sections.as_slice())
            .unwrap_or(&[])
    }

    pub fn active_entries(&self) -> &[Entry] {
        self.repo
            .as_ref()
            .and_then(|repo| repo.sections.get(self.active_section))
            .map(|section| section.entries.as_slice())
            .unwrap_or(&[])
    }

    pub fn section_title(&self) -> Option<&str> {
        self.sections()
            .get(self.active_section)
            .map(|section| section.title.as_str())
    }

    pub fn content_title(&self) -> String {
        if let Some(document) = &self.loaded_document {
            if document.relative_path.as_os_str().is_empty() {
                return format!("内容 · {}", document.title);
            }

            return format!(
                "内容 · {} [{}]",
                document.title,
                document.relative_path.display()
            );
        }

        "内容".to_string()
    }

    pub fn rendered_content_text(&mut self, width: usize) -> ratatui::text::Text<'static> {
        let width = width.max(12);
        if let Some(document) = self.loaded_document.as_mut() {
            return document.ensure_cache(width).text.clone();
        }

        let placeholder = if let Some(error) = &self.error {
            format!("读取失败：{error}")
        } else if self.sections().is_empty() {
            "docs/ 中没有可浏览的 Markdown 文档。".to_string()
        } else if self.active_entries().is_empty() {
            "No documents".to_string()
        } else {
            "按 Enter 打开当前文档。".to_string()
        };

        let parsed = tui_markdown::from_str(&placeholder);
        let lines: Vec<ratatui::text::Line<'static>> = parsed
            .lines
            .into_iter()
            .map(|line| {
                let spans: Vec<ratatui::text::Span<'static>> = line
                    .spans
                    .into_iter()
                    .map(|span| {
                        ratatui::text::Span::styled(span.content.to_string(), span.style)
                    })
                    .collect();
                ratatui::text::Line::from(spans)
            })
            .collect();
        ratatui::text::Text::from(lines)
    }

    pub fn toggle_dual_column(&mut self) {
        self.content_dual_column = !self.content_dual_column;
        self.content_scroll = 0;
    }

    pub fn sync_content_layout(&mut self, width: u16, height: u16) {
        self.content_viewport_height = height;
        if let Some(document) = self.loaded_document.as_mut() {
            document.ensure_cache(width as usize);
        }
        self.clamp_scroll(width as usize);
    }

    pub fn rendered_content_lines(&mut self, width: usize) -> Vec<String> {
        let text = self.rendered_content_text(width);
        text.lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    pub fn focus_left(&mut self) {
        self.focus = match self.focus {
            ManualFocus::Sections => ManualFocus::Sections,
            ManualFocus::Entries => ManualFocus::Sections,
            ManualFocus::Content => ManualFocus::Entries,
        };
    }

    pub fn focus_right(&mut self) {
        self.focus = match self.focus {
            ManualFocus::Sections => ManualFocus::Entries,
            ManualFocus::Entries => ManualFocus::Content,
            ManualFocus::Content => ManualFocus::Content,
        };
    }

    pub fn move_up(&mut self) {
        match self.focus {
            ManualFocus::Sections => {
                let len = self.sections().len();
                move_index(&mut self.section_cursor, len, -1);
            }
            ManualFocus::Entries => {
                let len = self.active_entries().len();
                move_index(&mut self.entry_cursor, len, -1);
            }
            ManualFocus::Content => {
                self.content_scroll = self.content_scroll.saturating_sub(1);
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.focus {
            ManualFocus::Sections => {
                let len = self.sections().len();
                move_index(&mut self.section_cursor, len, 1);
            }
            ManualFocus::Entries => {
                let len = self.active_entries().len();
                move_index(&mut self.entry_cursor, len, 1);
            }
            ManualFocus::Content => {
                self.content_scroll = self.content_scroll.saturating_add(1);
                self.clamp_scroll(self.cached_content_width());
            }
        }
    }

    pub fn confirm_focus(&mut self) {
        match self.focus {
            ManualFocus::Sections => self.activate_section(self.section_cursor),
            ManualFocus::Entries => self.open_entry(self.entry_cursor),
            ManualFocus::Content => {}
        }
    }

    fn activate_section(&mut self, index: usize) {
        if self.sections().is_empty() {
            self.loaded_document = None;
            self.active_entry = None;
            return;
        }

        self.active_section = index.min(self.sections().len().saturating_sub(1));
        self.section_cursor = self.active_section;
        self.entry_cursor = 0;
        self.active_entry = None;
        self.loaded_document = None;
        self.content_scroll = 0;
        self.content_dual_column = false;

        if !self.active_entries().is_empty() {
            self.open_entry(0);
        }
    }

    fn open_entry(&mut self, index: usize) {
        let Some(entry) = self.active_entries().get(index).cloned() else {
            self.active_entry = None;
            self.loaded_document = None;
            return;
        };

        self.entry_cursor = index;
        self.active_entry = Some(index);
        self.loaded_document = Some(LoadedDocument::from_entry(&entry));
        self.content_scroll = 0;
        self.content_dual_column = false;
        self.clamp_scroll(self.cached_content_width());
    }

    fn clamp_scroll(&mut self, width: usize) {
        let width = width.max(12);
        let viewport_height = self.content_viewport_height.max(1) as usize;
        let total_lines = if let Some(document) = self.loaded_document.as_mut() {
            document.ensure_cache(width).text.lines.len()
        } else {
            self.rendered_content_text(width).lines.len()
        };
        let max_scroll = total_lines.saturating_sub(viewport_height);
        self.content_scroll = self.content_scroll.min(max_scroll as u16);
    }

    fn cached_content_width(&self) -> usize {
        self.loaded_document
            .as_ref()
            .and_then(|document| document.render_cache.as_ref().map(|cache| cache.width))
            .unwrap_or(12)
    }
}

pub fn run_app(terminal: &mut AppTerminal, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &mut app))?;
        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                app.on_key_event(key);
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn placeholder_document(message: impl Into<String>) -> DocumentContent {
    DocumentContent {
        blocks: vec![Block::Placeholder(message.into())],
    }
}

fn move_index(index: &mut usize, len: usize, delta: isize) {
    if len == 0 {
        *index = 0;
        return;
    }

    let next = (*index as isize + delta).clamp(0, len.saturating_sub(1) as isize);
    *index = next as usize;
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use crate::manual::validate_repo_root;

    use super::{ManualFocus, ManualState};

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn validates_directory_with_docs_subdirectory() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("docs")).expect("create docs");

        assert!(validate_repo_root(temp.path()).is_ok());
    }

    #[test]
    fn rejects_directory_without_docs_subdirectory() {
        let temp = tempdir().expect("tempdir");

        assert!(validate_repo_root(temp.path()).is_err());
    }

    #[test]
    fn changing_section_resets_entries_and_opens_first_document() {
        let mut state = ManualState::new(fixture_root("manual_repo"));

        state.focus = ManualFocus::Sections;
        state.move_down();
        state.move_down();
        state.confirm_focus();

        assert_eq!(state.section_title(), Some("others"));
        assert_eq!(state.entry_cursor, 0);
        assert_eq!(state.active_entry, Some(0));
        assert_eq!(
            state
                .loaded_document
                .as_ref()
                .map(|document| document.title.as_str()),
            Some("配套 App")
        );
    }

    #[test]
    fn changing_entry_refreshes_loaded_content() {
        let mut state = ManualState::new(fixture_root("manual_repo"));

        state.focus = ManualFocus::Entries;
        state.move_down();
        state.confirm_focus();

        let lines = state.rendered_content_lines(32);
        assert_eq!(
            state
                .loaded_document
                .as_ref()
                .map(|document| document.title.as_str()),
            Some("入门")
        );
        assert!(lines.iter().any(|line| line.contains("快速开始")));
    }

    #[test]
    fn scroll_offset_clamps_to_rendered_content_height() {
        let mut state = ManualState::new(fixture_root("manual_repo"));

        state.focus = ManualFocus::Entries;
        state.move_down();
        state.confirm_focus();
        state.sync_content_layout(18, 2);
        state.content_scroll = 999;
        state.sync_content_layout(18, 2);

        let lines = state.rendered_content_lines(18);
        let expected_max = lines.len().saturating_sub(2) as u16;
        assert_eq!(state.content_scroll, expected_max);
    }
}
