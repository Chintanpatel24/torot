use crate::core::types::ToolStatus;
use crate::tui::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_tools(
    f: &mut Frame,
    area: Rect,
    tools: &[ToolStatus],
    search: &str,
) {
    let search_lower = search.to_lowercase();
    let filtered: Vec<&ToolStatus> = if search_lower.is_empty() {
        tools.iter().collect()
    } else {
        tools
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&search_lower) || t.description.to_lowercase().contains(&search_lower))
            .collect()
    };

    let installed = filtered.iter().filter(|t| t.installed).count();
    let total = filtered.len();

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Tool Registry — {installed}/{total} installed"),
            Style::new().fg(theme::ACCENT),
        )),
        Line::from(Span::styled(
            format!(" Search: {}█", search),
            Style::new().fg(theme::FG_SECONDARY),
        )),
        Line::from(""),
    ];

    for tool in filtered {
        let dot = if tool.installed {
            Span::styled(" ● ", Style::new().fg(theme::GREEN))
        } else {
            Span::styled(" ○ ", Style::new().fg(theme::FG_DIM))
        };

        let name_style = if tool.enabled && tool.installed {
            Style::new().fg(theme::FG_PRIMARY)
        } else {
            Style::new().fg(theme::FG_MUTED)
        };

        let version_str = if !tool.version.is_empty() {
            tool.version.clone()
        } else {
            tool.install_hint.clone()
        };

        lines.push(Line::from(vec![
            dot,
            Span::styled(format!("{:12}", tool.name), name_style),
            Span::styled(format!(" {:8}", tool.domain), Style::new().fg(theme::FG_DIM)),
            Span::styled(format!(" {}", version_str), Style::new().fg(theme::FG_SECONDARY)),
            if tool.enabled {
                Span::styled("  enabled ", Style::new().fg(theme::GREEN))
            } else {
                Span::styled("  disabled", Style::new().fg(theme::FG_DIM))
            },
        ]));
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
