use std::sync::OnceLock;

use figlet_rs::FIGlet;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, AppMode, ManualFocus, StatusKind, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH};

const PATH_PROMPT_BANNER: &[&str] = &["启梦 SCUT", "连接华工智慧，助力学习科研"];
const HOME_FIGLET_TEXT: &str = "Qimeng SCUT CLI";
const HOME_SUBTITLE_LINES: &[&str] = &["启梦·SCUT CLI", "连接华工智慧，助力学习科研"];
const HOME_COMPACT_BANNER: &[&str] = &[
    "启梦 SCUT CLI 感",
    "启梦·SCUT CLI",
    "连接华工智慧，助力学习科研",
];
static HOME_ASCII_BANNER: OnceLock<Option<HomeBannerData>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomeBannerVariant {
    Ascii,
    Compact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HomeBannerData {
    lines: Vec<String>,
    width: u16,
    height: u16,
}

pub fn render(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Clear, frame.area());
    match app.mode {
        AppMode::PathPrompt => render_path_prompt(frame, app),
        AppMode::Home => render_home(frame, app),
        AppMode::Manual => render_manual(frame, app),
    }
}

fn render_path_prompt(frame: &mut Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(6),
        ])
        .split(frame.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled("Qimeng SCUT", Style::default().fg(Color::Cyan)),
        Span::raw(" · 手册路径初始化"),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let banner = Paragraph::new(PATH_PROMPT_BANNER.join("\n"))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(banner, layout[1]);

    let body = Paragraph::new(vec![
        Line::from("默认路径不可用，请输入一个本地手册仓库路径。"),
        Line::from("仓库必须包含 `docs/` 子目录。"),
        Line::from("按 `Enter` 验证，按 `q` 退出。"),
    ])
    .block(Block::default().title("说明").borders(Borders::ALL))
    .wrap(Wrap { trim: false });
    frame.render_widget(body, centered_rect(80, 40, layout[2]));

    let input_area = centered_rect(80, 28, layout[3]);
    render_input_box(
        frame,
        input_area,
        "手册仓库路径",
        app.path_input.value(),
        app.path_input.cursor_offset(),
    );

    if let Some(status) = &app.status {
        let status_area = Rect {
            x: input_area.x,
            y: input_area.y.saturating_sub(3),
            width: input_area.width,
            height: 3,
        };
        render_status(
            frame,
            status_area,
            &status.text,
            status.kind == StatusKind::Error,
        );
    }
}

fn render_home(frame: &mut Frame, app: &App) {
    let banner_variant = select_home_banner(frame.area().width);
    let banner_height = home_banner_height(banner_variant);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_height),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let banner = Paragraph::new(home_banner_text(banner_variant)).alignment(Alignment::Left);
    frame.render_widget(banner, layout[0]);

    let meta = Paragraph::new(vec![
        Line::from(format!("版本：{}", env!("CARGO_PKG_VERSION"))),
        Line::from("输入 `manual` 浏览本地手册，输入 `q` 退出程序。"),
    ])
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(meta, layout[1]);

    let repo = Paragraph::new(app.manual_repo_path.display().to_string())
        .block(
            Block::default()
                .title("当前手册仓库路径")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(repo, layout[2]);

    if let Some(status) = &app.status {
        render_status(
            frame,
            layout[3],
            &status.text,
            matches!(status.kind, StatusKind::Error),
        );
    }

    render_input_box(
        frame,
        layout[4],
        "命令输入",
        &format!("qimeng-scut> {}", app.command_input.value()),
        "qimeng-scut> ".len() + app.command_input.cursor_offset(),
    );
}

fn render_manual(frame: &mut Frame, app: &mut App) {
    let Some(manual) = app.manual_state.as_mut() else {
        render_status(frame, frame.area(), "手册状态未初始化。", true);
        return;
    };

    if frame.area().width < MIN_TERMINAL_WIDTH || frame.area().height < MIN_TERMINAL_HEIGHT {
        render_resize_message(frame);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Qimeng SCUT",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" · Manual"),
        ]),
        Line::from(format!("仓库路径：{}", manual.repo_path.display())),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, layout[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ])
        .split(layout[1]);

    render_section_pane(frame, columns[0], manual);
    render_entry_pane(frame, columns[1], manual);
    render_content_pane(frame, columns[2], manual);

    let footer =
        Paragraph::new("q: 退出  Esc: 返回首页  ←/→: 切换栏位  ↑/↓: 移动或滚动  Enter: 确认")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
    frame.render_widget(footer, layout[2]);
}

fn render_section_pane(frame: &mut Frame, area: Rect, manual: &ManualFocusState) {
    let title = "章节";
    let items = if manual.sections().is_empty() {
        vec![ListItem::new("No sections")]
    } else {
        manual
            .sections()
            .iter()
            .enumerate()
            .map(|(index, section)| {
                let marker = if index == manual.active_section {
                    "* "
                } else {
                    "  "
                };
                ListItem::new(format!("{marker}{}", section.title))
            })
            .collect::<Vec<_>>()
    };

    render_list(
        frame,
        area,
        title,
        items,
        manual
            .section_cursor
            .min(manual.sections().len().saturating_sub(1)),
        manual.focus == ManualFocus::Sections,
    );
}

fn render_entry_pane(frame: &mut Frame, area: Rect, manual: &ManualFocusState) {
    let title = match manual.section_title() {
        Some(section_title) => format!("文档 · {section_title}"),
        None => "文档".to_string(),
    };

    let items = if manual.active_entries().is_empty() {
        vec![ListItem::new("No documents")]
    } else {
        manual
            .active_entries()
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let marker = if manual.active_entry == Some(index) {
                    "* "
                } else {
                    "  "
                };
                ListItem::new(format!(
                    "{marker}{} ({})",
                    entry.title,
                    entry.relative_path.display()
                ))
            })
            .collect::<Vec<_>>()
    };

    render_list(
        frame,
        area,
        &title,
        items,
        manual
            .entry_cursor
            .min(manual.active_entries().len().saturating_sub(1)),
        manual.focus == ManualFocus::Entries,
    );
}

type ManualFocusState = crate::app::ManualState;

fn render_content_pane(frame: &mut Frame, area: Rect, manual: &mut ManualFocusState) {
    let title = manual.content_title();
    let block = pane_block(&title, manual.focus == ManualFocus::Content);
    let inner = block.inner(area);
    manual.sync_content_layout(inner.width, inner.height);
    let text = manual.rendered_content_text(inner.width as usize);
    let paragraph = Paragraph::new(text)
        .block(block)
        .scroll((manual.content_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_list(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: Vec<ListItem<'static>>,
    selected: usize,
    focused: bool,
) {
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(selected.min(items.len().saturating_sub(1))));
    }

    let highlight_style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let list = List::new(items)
        .block(pane_block(title, focused))
        .highlight_style(highlight_style)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_status(frame: &mut Frame, area: Rect, text: &str, is_error: bool) {
    let color = if is_error {
        Color::LightRed
    } else {
        Color::LightGreen
    };
    let title = if is_error {
        "状态 · 错误"
    } else {
        "状态"
    };
    let status = Paragraph::new(text)
        .style(Style::default().fg(color))
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(status, area);
}

fn render_input_box(frame: &mut Frame, area: Rect, title: &str, value: &str, cursor: usize) {
    let paragraph = Paragraph::new(value.to_string())
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    frame.set_cursor_position((area.x + cursor as u16 + 1, area.y + 1));
}

fn render_resize_message(frame: &mut Frame) {
    let area = centered_rect(70, 30, frame.area());
    let body = Paragraph::new(vec![
        Line::from("终端窗口过小，无法显示三栏手册布局。"),
        Line::from(format!(
            "请将窗口调整到至少 {}x{}。",
            MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
        )),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().title("需要更大窗口").borders(Borders::ALL))
    .wrap(Wrap { trim: false });
    frame.render_widget(body, area);
}

fn select_home_banner(width: u16) -> HomeBannerVariant {
    match load_home_ascii_banner() {
        Some(banner) if width >= banner.width.saturating_add(2) => HomeBannerVariant::Ascii,
        _ => HomeBannerVariant::Compact,
    }
}

fn home_banner_height(variant: HomeBannerVariant) -> u16 {
    match variant {
        HomeBannerVariant::Ascii => load_home_ascii_banner()
            .map(|banner| banner.height + HOME_SUBTITLE_LINES.len() as u16 + 1)
            .unwrap_or(HOME_COMPACT_BANNER.len() as u16),
        HomeBannerVariant::Compact => HOME_COMPACT_BANNER.len() as u16,
    }
}

fn home_banner_text(variant: HomeBannerVariant) -> Text<'static> {
    match variant {
        HomeBannerVariant::Ascii => load_home_ascii_banner()
            .map(ascii_banner_text)
            .unwrap_or_else(compact_banner_text),
        HomeBannerVariant::Compact => Text::from(vec![
            Line::from(Span::styled(
                HOME_COMPACT_BANNER[0],
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                HOME_COMPACT_BANNER[1],
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                HOME_COMPACT_BANNER[2],
                Style::default().fg(Color::Cyan),
            )),
        ]),
    }
}

fn load_home_ascii_banner() -> Option<&'static HomeBannerData> {
    HOME_ASCII_BANNER
        .get_or_init(build_home_ascii_banner)
        .as_ref()
}

fn build_home_ascii_banner() -> Option<HomeBannerData> {
    let font = FIGlet::slant().ok()?;
    let figure = font.convert(HOME_FIGLET_TEXT)?;
    let mut lines = figure
        .as_str()
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return None;
    }

    let width = lines
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()) as u16)
        .max()
        .unwrap_or(0);

    Some(HomeBannerData {
        width,
        height: lines.len() as u16,
        lines,
    })
}

fn ascii_banner_text(banner: &HomeBannerData) -> Text<'static> {
    let mut lines = banner
        .lines
        .iter()
        .cloned()
        .map(|line| {
            Line::from(Span::styled(
                line,
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect::<Vec<_>>();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        HOME_SUBTITLE_LINES[0],
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        HOME_SUBTITLE_LINES[1],
        Style::default().fg(Color::Cyan),
    )));

    Text::from(lines)
}

fn compact_banner_text() -> Text<'static> {
    Text::from(vec![
        Line::from(Span::styled(
            HOME_COMPACT_BANNER[0],
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            HOME_COMPACT_BANNER[1],
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            HOME_COMPACT_BANNER[2],
            Style::default().fg(Color::Cyan),
        )),
    ])
}

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::{
        home_banner_height, load_home_ascii_banner, select_home_banner, HomeBannerVariant,
    };

    #[test]
    fn loads_slant_banner_successfully() {
        let banner = load_home_ascii_banner().expect("slant banner should load");

        assert!(banner.width > 0);
        assert!(banner.height > 0);
        assert!(banner.lines.len() > 1);
    }

    #[test]
    fn uses_ascii_banner_when_terminal_is_wide_enough() {
        let banner = load_home_ascii_banner().expect("slant banner should load");

        assert_eq!(
            select_home_banner(banner.width.saturating_add(2)),
            HomeBannerVariant::Ascii
        );
    }

    #[test]
    fn falls_back_to_compact_banner_when_terminal_is_narrow() {
        let banner = load_home_ascii_banner().expect("slant banner should load");

        assert_eq!(
            select_home_banner(banner.width.saturating_add(1)),
            HomeBannerVariant::Compact
        );
        assert!(
            home_banner_height(HomeBannerVariant::Ascii)
                > home_banner_height(HomeBannerVariant::Compact)
        );
    }
}
