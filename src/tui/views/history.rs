use crate::core::types::DbSession;
use crate::tui::theme;
use crate::util::time::format_timestamp;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render_history(
    f: &mut Frame,
    area: Rect,
    sessions: &[DbSession],
) {
    if sessions.is_empty() {
        f.render_widget(
            Paragraph::new(" No past sessions.")
                .style(Style::new().fg(theme::FG_MUTED)),
            area,
        );
        return;
    }

    let header_cells = ["ID", "Target", "Started", "Findings", "Summary"];
    let header = Row::new(
        header_cells
            .iter()
            .map(|c| Cell::from(*c).style(Style::new().fg(theme::FG_SECONDARY))),
    );

    let rows: Vec<Row> = sessions
        .iter()
        .map(|s| {
            let finding_color = if s.total_findings > 0 {
                theme::ORANGE
            } else {
                theme::FG_MUTED
            };

            Row::new(vec![
                Cell::from(s.id.clone()).style(Style::new().fg(theme::FG_SECONDARY)),
                Cell::from(s.target.clone()).style(Style::new().fg(theme::FG_PRIMARY)),
                Cell::from(format_timestamp(s.start_time)).style(Style::new().fg(theme::FG_MUTED)),
                Cell::from(s.total_findings.to_string())
                    .style(Style::new().fg(finding_color)),
                Cell::from(s.summary.clone()).style(Style::new().fg(theme::FG_SECONDARY)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::BORDER)),
    );

    f.render_widget(table, area);
}
