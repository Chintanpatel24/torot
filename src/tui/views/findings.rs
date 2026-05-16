use crate::core::types::Finding;
use crate::tui::theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct FindingDetail {
    pub id: String,
    pub tool: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub file: String,
    pub line: u32,
    pub fix_suggestion: String,
    pub impact: String,
}

pub fn render_findings(
    f: &mut Frame,
    area: Rect,
    findings: &[Finding],
    detail: Option<&FindingDetail>,
) {
    if findings.is_empty() {
        f.render_widget(
            Paragraph::new(" No findings. Run a scan first.")
                .style(Style::new().fg(theme::FG_MUTED)),
            area,
        );
        return;
    }

    if let Some(d) = detail {
        render_finding_detail(f, area, d);
    } else {
        render_finding_list(f, area, findings);
    }
}

fn render_finding_list(f: &mut Frame, area: Rect, findings: &[Finding]) {
    let items: Vec<Line> = findings
        .iter()
        .map(|f2| {
            Line::from(vec![
                Span::styled(
                    format!("{:9}", f2.severity),
                    theme::SeverityColors::style_bold(&f2.severity),
                ),
                Span::styled(
                    format!(" {:12}", f2.tool),
                    theme::tool_name_style(),
                ),
                Span::styled(
                    format!(" {}", f2.title),
                    Style::new().fg(theme::FG_PRIMARY),
                ),
            ])
        })
        .collect();

    f.render_widget(
        Paragraph::new(Text::from(items))
            .block(
                Block::default()
                    .title(format!(" Findings — {} total ", findings.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(theme::BORDER)),
            ),
        area,
    );
}

fn render_finding_detail(f: &mut Frame, area: Rect, d: &FindingDetail) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" [{}]", d.severity),
                theme::SeverityColors::style_bold(&d.severity),
            ),
            Span::styled(
                format!(" {} — {}", d.tool, d.title),
                Style::new().fg(theme::FG_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if !d.file.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" File: {}:{}", d.file, d.line),
            Style::new().fg(theme::FG_SECONDARY),
        )));
    }

    if !d.description.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Description:",
            Style::new().fg(theme::FG_MUTED).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            &d.description,
            Style::new().fg(theme::FG_PRIMARY),
        )));
    }

    if !d.impact.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Impact:",
            Style::new().fg(theme::FG_MUTED).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            &d.impact,
            Style::new().fg(theme::ORANGE),
        )));
    }

    if !d.fix_suggestion.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Fix:",
            Style::new().fg(theme::GREEN).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            &d.fix_suggestion,
            Style::new().fg(theme::FG_PRIMARY),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " [Esc] back to list",
        Style::new().fg(theme::FG_MUTED),
    )));

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
