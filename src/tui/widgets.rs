use crate::tui::theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};

pub fn render_titlebar(f: &mut Frame, area: Rect, title: &str, stats: &str) {
    let content = if !stats.is_empty() {
        format!(" {}  │  {}", title, stats)
    } else {
        format!(" {}", title)
    };

    f.render_widget(
        Paragraph::new(content)
            .style(theme::titlebar_style()),
        area,
    );
}

pub fn render_statusbar(f: &mut Frame, area: Rect, mode: &str, extra: &str) {
    let left = format!(" {} ", mode);
    let right = if !extra.is_empty() {
        format!(" {} ", extra)
    } else {
        String::new()
    };

    let padding = area.width.saturating_sub(left.len() as u16 + right.len() as u16 + 2);
    let mid = " ".repeat(padding.saturating_sub(1) as usize);
    let content = format!("{}{}{}", left, mid, right);

    f.render_widget(
        Paragraph::new(content).style(theme::statusbar_style()),
        area,
    );
}

pub fn render_logo(f: &mut Frame, area: Rect, text: &str) {
    f.render_widget(
        Paragraph::new(text)
            .style(Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_info_block(f: &mut Frame, area: Rect, lines: &[Line]) {
    let widget = Paragraph::new(Text::from(lines.to_vec()))
        .block(theme::block_style());
    f.render_widget(widget, area);
}

pub fn render_empty(f: &mut Frame, area: Rect, message: &str) {
    f.render_widget(
        Paragraph::new(message)
            .style(Style::new().fg(theme::FG_MUTED))
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_spinner(done: usize, total: usize) -> String {
    let pct = if total > 0 { (done as f64 / total as f64) * 100.0 } else { 0.0 };
    let filled = (pct / 10.0) as usize;
    let empty = 10usize.saturating_sub(filled);
    format!(
        "[{}{}] {}/{}",
        "█".repeat(filled),
        "░".repeat(empty),
        done,
        total,
    )
}

pub fn severity_badge(sev: &str) -> Span<'_> {
    let (color, bg) = match sev {
        "CRITICAL" => (theme::RED, theme::BG_LIGHT),
        "HIGH" => (theme::ORANGE, theme::BG_LIGHT),
        "MEDIUM" => (theme::YELLOW, theme::BG_LIGHT),
        _ => (theme::FG_MUTED, theme::BG_LIGHT),
    };
    Span::styled(
        format!(" {} ", sev),
        Style::new().fg(color).bg(bg).add_modifier(Modifier::BOLD),
    )
}

#[allow(dead_code)]
pub fn tool_status_span(name: &str, installed: bool, enabled: bool) -> Span<'static> {
    let style = if enabled {
        Style::new().fg(if installed { theme::GREEN } else { theme::FG_DIM })
    } else {
        Style::new().fg(theme::FG_MUTED)
    };
    Span::styled(format!(" ● {name:<12}"), style)
}
