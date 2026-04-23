use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{input::TextInput, ui};

pub type AppTerminal = Terminal<CrosstermBackend<std::io::Stdout>>;

pub const DEFAULT_MANUAL_REPO: &str = "/home/xuyang/code/survive-in-scut";

const TICK_RATE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    PathPrompt,
    Home,
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

#[derive(Debug)]
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub manual_repo_path: PathBuf,
    pub command_input: TextInput,
    pub path_input: TextInput,
    pub status: Option<StatusMessage>,
}

impl App {
    pub fn new() -> Self {
        let default_path = PathBuf::from(DEFAULT_MANUAL_REPO);
        let default_input = default_path.display().to_string();

        if is_valid_manual_repo(&default_path) {
            Self {
                mode: AppMode::Home,
                should_quit: false,
                manual_repo_path: default_path,
                command_input: TextInput::default(),
                path_input: TextInput::new(default_input),
                status: Some(StatusMessage {
                    kind: StatusKind::Info,
                    text: "输入 `manual` 进入手册浏览模式。".to_string(),
                }),
            }
        } else {
            Self {
                mode: AppMode::PathPrompt,
                should_quit: false,
                manual_repo_path: default_path,
                command_input: TextInput::default(),
                path_input: TextInput::new(default_input),
                status: Some(StatusMessage {
                    kind: StatusKind::Error,
                    text: "默认手册仓库无效，请输入一个包含 docs/ 的本地路径。".to_string(),
                }),
            }
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
        }
    }

    fn handle_path_prompt(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Enter) {
            let candidate = PathBuf::from(self.path_input.value().trim());
            if is_valid_manual_repo(&candidate) {
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
                    self.status = Some(StatusMessage {
                        kind: StatusKind::Info,
                        text: "Manual 模式将在下一阶段接入。".to_string(),
                    });
                }
                "" => {}
                other => {
                    self.status = Some(StatusMessage {
                        kind: StatusKind::Error,
                        text: format!("未知命令：`{other}`。当前仅支持 `manual`。"),
                    });
                }
            }
            self.command_input.clear();
            return;
        }

        self.command_input.handle_key(key);
    }
}

pub fn run_app(terminal: &mut AppTerminal, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;
        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                app.on_key_event(key);
            }
        }
    }

    Ok(())
}

fn is_valid_manual_repo(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_dir() {
        return false;
    }

    fs::metadata(path.join("docs"))
        .map(|docs_metadata| docs_metadata.is_dir())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::is_valid_manual_repo;

    #[test]
    fn validates_directory_with_docs_subdirectory() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("docs")).expect("create docs");

        assert!(is_valid_manual_repo(temp.path()));
    }

    #[test]
    fn rejects_directory_without_docs_subdirectory() {
        let temp = tempdir().expect("tempdir");

        assert!(!is_valid_manual_repo(temp.path()));
    }
}
