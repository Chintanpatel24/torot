use crate::tui::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct SettingsState {
    pub install_mode: String,
    pub sandbox_profile: String,
    pub max_runtime: u64,
    pub report_template: String,
    pub saved: bool,
    pub message: String,
}

pub fn render_settings(
    f: &mut Frame,
    area: Rect,
    s: &SettingsState,
) {
    let mut lines = vec![
        Line::from(Span::styled(" Settings", Style::new().fg(theme::ACCENT))),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Install mode: ", Style::new().fg(theme::FG_PRIMARY)),
            Span::styled(&s.install_mode, Style::new().fg(theme::FG_SECONDARY)),
            Span::styled("  [set in config file]", Style::new().fg(theme::FG_MUTED)),
        ]),
        Line::from(vec![
            Span::styled(" Sandbox profile: ", Style::new().fg(theme::FG_PRIMARY)),
            Span::styled(&s.sandbox_profile, Style::new().fg(theme::FG_SECONDARY)),
            Span::styled("  [1 to toggle]", Style::new().fg(theme::FG_MUTED)),
        ]),
        Line::from(vec![
            Span::styled(" Max runtime: ", Style::new().fg(theme::FG_PRIMARY)),
            Span::styled(format!("{}s", s.max_runtime), Style::new().fg(theme::FG_SECONDARY)),
            Span::styled("  [2 to cycle]", Style::new().fg(theme::FG_MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Report template:", Style::new().fg(theme::FG_MUTED))),
    ];

    for line in s.report_template.lines().take(8) {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            Style::new().fg(theme::FG_SECONDARY),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " [s] Save settings",
        Style::new().fg(theme::GREEN),
    )));

    if s.saved {
        lines.push(Line::from(Span::styled(
            " Saved ✓",
            Style::new().fg(theme::GREEN),
        )));
    }

    if !s.message.is_empty() && !s.message.starts_with("Report") {
        lines.push(Line::from(Span::styled(&s.message, Style::new().fg(theme::GREEN))));
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(theme::BORDER)),
            ),
        area,
    );
}
