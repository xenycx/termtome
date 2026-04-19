use crate::app::App;
use crate::ui::theme::get_theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if !app.show_help {
        return;
    }

    let theme = get_theme(app.theme);
    let area = centered_rect(60, 60, f.area());

    let help_text = vec![
        Line::from("Keyboard Shortcuts:"),
        Line::from(""),
        Line::from("Global:"),
        Line::from("  q          - Quit application"),
        Line::from("  Tab        - Switch panes (Library/Reader/Downloader)"),
        Line::from("  :          - Open command mode"),
        Line::from("  /          - Search in book"),
        Line::from("  t          - Toggle Light/Dark Theme"),
        Line::from("  P          - Open Preferences Menu"),
        Line::from("  Ctrl+H / ? - Toggle this Help Menu"),
        Line::from("  +/-        - Increase/Decrease margin width"),
        Line::from(""),
        Line::from("Navigation (Library/Downloader):"),
        Line::from("  j/k, Up/Down - Move selection up/down"),
        Line::from("  Enter        - Open book / Download book"),
        Line::from(""),
        Line::from("Reader:"),
        Line::from("  c          - Table of Contents (TOC)"),
        Line::from("  j/k, Up/Down - Scroll up/down"),
        Line::from("  f/b, PgDn/Up - Fast scroll"),
        Line::from("  n/p          - Next/Previous Chapter"),
        Line::from("  n/N          - Next/Previous Search Result"),
        Line::from("  m            - Add Bookmark"),
        Line::from(""),
        Line::from("Preferences Menu:"),
        Line::from("  j/k, Up/Down - Move selection"),
        Line::from("  Enter/Space  - Toggle setting or cycle value"),
        Line::from("  +/-          - Adjust numeric values"),
        Line::from("  Esc/q/P      - Close and save"),
        Line::from(""),
        Line::from("Commands:"),
        Line::from("  :import <path>      - Import EPUB/TXT file"),
        Line::from("  :delete [book]      - Delete selected/named book"),
        Line::from("  :move <bk> <cat>    - Move book to category"),
        Line::from("  :create <category>  - Create new category"),
        Line::from("  :dl / :downloader   - Open online downloader"),
        Line::from("  :theme [light/dark] - Toggle/Set theme"),
        Line::from("  :quit / :q          - Exit application"),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.background).fg(theme.foreground));

    let p = Paragraph::new(help_text)
        .block(block)
        .scroll((app.help_scroll, 0))
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
