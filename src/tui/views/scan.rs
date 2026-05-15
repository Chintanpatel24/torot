use crate::core::types::Finding;
use crate::tui::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

pub struct ScanState {
    pub stream_lines: Vec<StreamLine>,
    pub findings: Vec<Finding>,
    pub running: bool,
    pub complete: bool,
    pub target: String,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub tab_selected: usize,
    pub generated_report_path: Option<String>,
}

#[derive(Clone)]
pub struct StreamLine {
    pub tool: String,
    pub line: String,
    pub kind: String,
    pub severity: Option<String>,
}

pub fn render_scan(
    f: &mut Frame,
    area: Rect,
    state: &ScanState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_scan_header(f, chunks[0], state);
    render_scan_tabs(f, chunks[2], state.tab_selected);

    if state.tab_selected == 0 {
        render_scan_output(f, chunks[1], state);
    } else {
        render_scan_findings_list(f, chunks[1], state);
    }
}

fn render_scan_header(f: &mut Frame, area: Rect, state: &ScanState) {
    let crit = state.findings.iter().filter(|f2| f2.severity == "CRITICAL").count();
    let high = state.findings.iter().filter(|f2| f2.severity == "HIGH").count();
    let status = if state.running { " RUNNING " } else if state.complete { " COMPLETE " } else { " IDLE " };

    let header = format!(
        "{status} target: {} | {} findings | {crit} crit {high} high",
        state.target,
        state.findings.len(),
    );
    f.render_widget(
        Paragraph::new(header)
            .style(Style::new().fg(theme::FG_PRIMARY).bg(theme::BG_LIGHT)),
        area,
    );
}

fn render_scan_tabs(f: &mut Frame, area: Rect, selected: usize) {
    let tab_names = vec![" Output ", " Findings "];
    let tabs = Tabs::new(
        tab_names
            .iter()
            .map(|t| Span::styled(*t, Style::new().fg(theme::FG_PRIMARY)))
            .collect::<Vec<_>>(),
    )
    .select(selected)
    .style(Style::new().bg(theme::BG_MEDIUM))
    .highlight_style(Style::new().bg(theme::ACCENT_DIM));
    f.render_widget(tabs, area);
}

fn render_scan_output(f: &mut Frame, area: Rect, state: &ScanState) {
    if state.stream_lines.is_empty() {
        f.render_widget(
            Paragraph::new(" Waiting for tool output...")
                .style(Style::new().fg(theme::FG_MUTED)),
            area,
        );
        return;
    }

    let max_lines = area.height as usize;
    let total = state.stream_lines.len();
    let scroll = if state.auto_scroll {
        total.saturating_sub(max_lines)
    } else {
        state.scroll_offset.min(total.saturating_sub(max_lines))
    };
    let end = (scroll + max_lines).min(total);
    let start = end.saturating_sub(max_lines);

    let lines: Vec<Line> = state.stream_lines[start..end]
        .iter()
        .map(|sl| {
            let style = if sl.kind == "system" {
                Style::new().fg(theme::CYAN)
            } else if let Some(ref sev) = sl.severity {
                theme::SeverityColors::style(sev)
            } else {
                Style::new().fg(theme::FG_PRIMARY)
            };

            Line::from(vec![
                Span::styled(
                    format!("{:12}", sl.tool.chars().take(12).collect::<String>()),
                    theme::tool_name_style(),
                ),
                Span::styled(&sl.line, style),
            ])
        })
        .collect();

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

fn render_scan_findings_list(f: &mut Frame, area: Rect, state: &ScanState) {
    if state.findings.is_empty() {
        f.render_widget(
            Paragraph::new(" No findings yet.")
                .style(Style::new().fg(theme::FG_MUTED)),
            area,
        );
        return;
    }

    let items: Vec<Line> = state
        .findings
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
                Span::styled(format!(" {}", f2.title), Style::new().fg(theme::FG_PRIMARY)),
            ])
        })
        .collect();

    f.render_widget(
        Paragraph::new(Text::from(items))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(theme::BORDER)),
            ),
        area,
    );
}
