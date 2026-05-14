use crate::tui::theme;
use crate::core::types::ToolStatus;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_home(
    f: &mut Frame,
    area: Rect,
    target_input: &str,
    mode: &str,
    selected_tools: &[String],
    tool_statuses: &[ToolStatus],
    show_advanced: bool,
    report_path: &str,
    _report_template: &str,
    launch_error: &str,
    is_editing: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let logo = format!(" TOROT v4 — bug bounty orchestration ");
    f.render_widget(
        Paragraph::new(logo)
            .style(Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center),
        chunks[0],
    );

    let mut lines: Vec<Line> = Vec::new();
    let cursor = if is_editing { "█" } else { "" };

    lines.push(Line::from(Span::styled(
        format!(" Target: {}{}", target_input, cursor),
        Style::new().fg(theme::FG_PRIMARY),
    )));
    lines.push(Line::from(""));

    let mode_help = " [1] single  [2] deep  [3] passive";
    lines.push(Line::from(vec![
        Span::styled(" Mode:", Style::new().fg(theme::FG_PRIMARY)),
        Span::styled(format!("   {}   {}", mode, mode_help), Style::new().fg(theme::FG_SECONDARY)),
    ]));
    lines.push(Line::from(""));

    let installed: Vec<&ToolStatus> = tool_statuses.iter().filter(|t| t.installed && t.enabled).collect();

    lines.push(Line::from(vec![
        Span::styled(" Tools:", Style::new().fg(theme::FG_PRIMARY)),
        Span::styled(
            format!(
                "  {} selected [space to toggle all]",
                if selected_tools.is_empty() {
                    "auto-detect".to_string()
                } else {
                    selected_tools.len().to_string()
                },
            ),
            Style::new().fg(theme::FG_SECONDARY),
        ),
    ]));

    if !installed.is_empty() {
        let tool_line: Vec<Span> = installed
            .iter()
            .map(|t| {
                let sel = selected_tools.is_empty() || selected_tools.contains(&t.name);
                Span::styled(
                    format!("{}{}  ", if sel { "[x]" } else { "[ ]" }, t.name),
                    if sel {
                        Style::new().fg(theme::GREEN)
                    } else {
                        Style::new().fg(theme::FG_MUTED)
                    },
                )
            })
            .collect();
        lines.push(Line::from(tool_line));
    } else {
        lines.push(Line::from(Span::styled(
            "  No tools detected. Install nmap, nuclei, semgrep, etc.",
            Style::new().fg(theme::FG_MUTED),
        )));
    }

    lines.push(Line::from(""));

    if show_advanced {
        lines.push(Line::from(Span::styled(" Advanced Options", Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD))));
        lines.push(Line::from(Span::styled(
            format!(" Report path:  {}", if report_path.is_empty() { "(default)" } else { report_path }),
            Style::new().fg(theme::FG_SECONDARY),
        )));
        lines.push(Line::from(Span::styled(
            " [a] Hide advanced",
            Style::new().fg(theme::FG_MUTED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            " [a] Advanced options",
            Style::new().fg(theme::FG_MUTED),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " [Enter] Launch scan",
        Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
    )));

    if !launch_error.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" ERROR: {launch_error}"),
            Style::new().fg(theme::RED),
        )));
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(theme::BORDER)),
            ),
        chunks[1],
    );
}
