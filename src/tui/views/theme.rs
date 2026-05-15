use ratatui::style::{Color, Modifier, Style};

pub const BG_DARK: Color = Color::Rgb(20, 20, 20);
pub const BG_MEDIUM: Color = Color::Rgb(30, 30, 30);
pub const BG_LIGHT: Color = Color::Rgb(40, 40, 40);
pub const BG_SURFACE: Color = Color::Rgb(25, 25, 25);
pub const FG_PRIMARY: Color = Color::Rgb(220, 220, 220);
pub const FG_SECONDARY: Color = Color::Rgb(160, 160, 160);
pub const FG_MUTED: Color = Color::Rgb(100, 100, 100);
pub const FG_DIM: Color = Color::Rgb(70, 70, 70);
pub const ACCENT: Color = Color::Rgb(255, 180, 0);
pub const ACCENT_DIM: Color = Color::Rgb(180, 120, 0);
pub const GREEN: Color = Color::Rgb(100, 200, 100);
pub const GREEN_BRIGHT: Color = Color::Rgb(140, 240, 140);
pub const RED: Color = Color::Rgb(255, 80, 80);
pub const ORANGE: Color = Color::Rgb(255, 140, 0);
pub const YELLOW: Color = Color::Rgb(255, 200, 60);
pub const BLUE: Color = Color::Rgb(80, 160, 255);
pub const CYAN: Color = Color::Rgb(100, 200, 255);
pub const BORDER: Color = Color::Rgb(60, 60, 60);

pub struct SeverityColors;

impl SeverityColors {
    pub fn style(sev: &str) -> Style {
        let color = match sev {
            "CRITICAL" => RED,
            "HIGH" => ORANGE,
            "MEDIUM" => YELLOW,
            "LOW" => FG_MUTED,
            _ => FG_SECONDARY,
        };
        Style::new().fg(color)
    }

    pub fn style_bold(sev: &str) -> Style {
        Self::style(sev).add_modifier(Modifier::BOLD)
    }
}

pub fn titlebar_style() -> Style {
    Style::new().bg(ACCENT_DIM).fg(Color::White).add_modifier(Modifier::BOLD)
}

pub fn statusbar_style() -> Style {
    Style::new().bg(BG_MEDIUM).fg(FG_SECONDARY)
}

pub fn sidebar_style(active: bool) -> Style {
    if active {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(FG_MUTED)
    }
}

pub fn tool_name_style() -> Style {
    Style::new().fg(GREEN)
}

pub fn block_style() -> ratatui::widgets::Block<'static> {
    ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::new().fg(BORDER))
}
