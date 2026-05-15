pub mod app;
pub mod theme;
pub mod widgets;
pub mod views;

pub use app::TuiApp;

#[derive(Clone, PartialEq)]
pub enum View {
    Home,
    Scan,
    Findings,
    History,
    Tools,
    Settings,
}

impl View {
    pub fn name(&self) -> &str {
        match self {
            View::Home => "HOME",
            View::Scan => "SCAN",
            View::Findings => "FINDINGS",
            View::History => "HISTORY",
            View::Tools => "TOOLS",
            View::Settings => "SETTINGS",
        }
    }

    pub fn idx(&self) -> usize {
        match self {
            View::Home => 0,
            View::Scan => 1,
            View::Findings => 2,
            View::History => 3,
            View::Tools => 4,
            View::Settings => 5,
        }
    }

    pub fn from_idx(i: usize) -> Self {
        match i {
            0 => View::Home,
            1 => View::Scan,
            2 => View::Findings,
            3 => View::History,
            4 => View::Tools,
            5 => View::Settings,
            _ => View::Home,
        }
    }
}
