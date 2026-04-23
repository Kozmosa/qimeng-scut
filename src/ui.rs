use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, StatusKind};

const BANNER: &[&str] = &["启梦 SCUT", "连接华工智慧，助力学习科研"];

pub fn render(frame: &mut Frame, app: &App) {
    frame.render_widget(Clear, frame.area());
    match app.mode {
        AppMode::PathPrompt => render_path_prompt(frame, app),
        AppMode::Home => render_home(frame, app),
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

    let banner = Paragraph::new(BANNER.join("\n"))
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
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let banner = Paragraph::new(vec![
        Line::from(Span::styled(
            "启梦 SCUT CLI 感",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("启梦 SCUT CLI"),
        Line::from("连接华工智慧，助力学习科研"),
    ])
    .alignment(Alignment::Left);
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
