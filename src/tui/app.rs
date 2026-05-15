use crate::core::{
    config::{get_settings, save_settings},
    engine::start_scan,
    event::{AppEvent, EventBus},
    report::generate_report,
    state::{
        get_db_stats, get_findings, get_sessions, stop_scan, AppState,
    },
    tools::get_tools,
    types::{
        AppConfig, Finding, ReportRequest, SandboxConfig, ScanRequest, ToolStatus,
    },
};
use crate::tui::views::{
    home::render_home,
    scan::{render_scan, ScanState, StreamLine},
    findings::{render_findings, FindingDetail},
    history::render_history,
    tools::render_tools,
    settings::{render_settings, SettingsState},
};
use crate::tui::{theme, View};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, TableState},
    Frame,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

pub struct TuiApp {
    state: Arc<AppState>,
    bus: EventBus,
    rx: Mutex<broadcast::Receiver<AppEvent>>,

    view: View,
    quit: bool,
    input_mode: InputMode,

    // home
    target_input: String,
    mode: String,
    selected_tools: Vec<String>,
    launch_error: String,
    report_path_input: String,
    report_template_input: String,
    show_advanced: bool,

    // scan
    scan_stream: Vec<StreamLine>,
    scan_findings: Vec<Finding>,
    scan_running: bool,
    scan_session_id: Option<String>,
    scan_complete: bool,
    generated_report_path: Option<String>,
    scroll_offset: usize,
    tab_selected: usize,
    auto_scroll: bool,

    // findings
    findings: Vec<Finding>,
    active_finding: Option<FindingDetail>,

    // history
    sessions: Vec<crate::core::types::DbSession>,

    // tools
    tool_statuses: Vec<ToolStatus>,
    tool_search: String,

    // settings
    settings: SettingsState,

    // global
    db_stats: serde_json::Value,
    tick_count: u64,
    _table_state: TableState,
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

impl TuiApp {
    pub fn new(
        state: Arc<AppState>,
        bus: EventBus,
        rx: broadcast::Receiver<AppEvent>,
    ) -> Self {
        let tools = get_tools(&state).unwrap_or_default();
        let stats = get_db_stats(&state);
        let config = get_settings(&state).ok();

        let mut s = SettingsState {
            install_mode: "both".to_string(),
            sandbox_profile: "strong".to_string(),
            max_runtime: 900,
            report_template: String::new(),
            saved: false,
            message: String::new(),
        };

        if let Some(c) = config {
            s.install_mode = c.install_mode;
            s.sandbox_profile = c.sandbox.profile;
            s.max_runtime = c.sandbox.max_runtime_seconds;
            s.report_template = c.default_report_template;
        }

        Self {
            state,
            bus,
            rx: Mutex::new(rx),
            view: View::Home,
            quit: false,
            input_mode: InputMode::Normal,
            target_input: String::new(),
            mode: "single".to_string(),
            selected_tools: vec![],
            launch_error: String::new(),
            report_path_input: String::new(),
            report_template_input: String::new(),
            show_advanced: false,
            scan_stream: vec![],
            scan_findings: vec![],
            scan_running: false,
            scan_session_id: None,
            scan_complete: false,
            generated_report_path: None,
            scroll_offset: 0,
            tab_selected: 0,
            auto_scroll: true,
            findings: vec![],
            active_finding: None,
            sessions: vec![],
            tool_statuses: tools,
            tool_search: String::new(),
            settings: s,
            db_stats: stats,
            tick_count: 0,
            _table_state: TableState::default(),
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> anyhow::Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(50);

        loop {
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
                }
            }

            self.tick();

            terminal.draw(|f| self.render(f))?;

            if self.quit {
                break;
            }
            last_tick = Instant::now();
        }
        Ok(())
    }

    fn tick(&mut self) {
        self.tick_count += 1;

        let events: Vec<AppEvent> = {
            let mut rx = match self.rx.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let mut ev = Vec::new();
            loop {
                match rx.try_recv() {
                    Ok(e) => ev.push(e),
                    Err(_) => break,
                }
            }
            ev
        };

        for event in events {
            match event {
                AppEvent::Line { tool, line, kind, severity } => {
                    self.scan_stream.push(StreamLine { tool, line, kind, severity });
                    if self.scan_stream.len() > 2000 {
                        self.scan_stream.remove(0);
                    }
                    if self.auto_scroll {
                        self.scroll_offset = self.scan_stream.len().saturating_sub(1);
                    }
                }
                AppEvent::Finding(f) => {
                    self.scan_findings.push(f);
                }
                AppEvent::ScanComplete { report_path } => {
                    self.scan_running = false;
                    self.scan_complete = true;
                    self.generated_report_path = report_path;
                    self.refresh_sessions();
                    self.refresh_stats();
                }
            }
        }

        if self.tick_count % 20 == 0 {
            self.refresh_stats();
        }
    }

    fn refresh_stats(&mut self) {
        self.db_stats = get_db_stats(&self.state);
    }

    fn refresh_sessions(&mut self) {
        self.sessions = get_sessions(&self.state).unwrap_or_default();
    }

    fn refresh_tools(&mut self) {
        self.tool_statuses = get_tools(&self.state).unwrap_or_default();
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Editing {
            self.handle_editing_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => self.quit = true,
            KeyCode::Char('1') => self.set_view(View::Home),
            KeyCode::Char('2') => self.set_view(View::Scan),
            KeyCode::Char('3') => self.set_view(View::Findings),
            KeyCode::Char('4') => self.set_view(View::History),
            KeyCode::Char('5') => self.set_view(View::Tools),
            KeyCode::Char('6') => self.set_view(View::Settings),
            KeyCode::Tab | KeyCode::Right => {
                let next = (self.view.idx() + 1).min(5);
                self.set_view(View::from_idx(next));
            }
            KeyCode::Left => {
                let prev = self.view.idx().saturating_sub(1);
                self.set_view(View::from_idx(prev));
            }
            KeyCode::Esc => self.set_view(View::Home),
            _ => match self.view {
                View::Home => self.handle_home_key(key),
                View::Scan => self.handle_scan_key(key),
                View::Findings => self.handle_findings_key(key),
                View::History => self.handle_history_key(key),
                View::Tools => self.handle_tools_key(key),
                View::Settings => self.handle_settings_key(key),
            },
        }
    }

    fn handle_editing_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                match self.view {
                    View::Home => self.launch_scan(),
                    _ => {}
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                match self.view {
                    View::Home => { self.target_input.pop(); }
                    View::Tools => { self.tool_search.pop(); }
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                match self.view {
                    View::Home => { self.target_input.push(c); }
                    View::Tools => { self.tool_search.push(c); }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn set_view(&mut self, v: View) {
        self.view = v.clone();
        match v {
            View::Findings => {
                let sid = self.scan_session_id.clone().unwrap_or_default();
                self.findings = get_findings(sid, &self.state);
            }
            View::History => self.refresh_sessions(),
            View::Tools => self.refresh_tools(),
            _ => {}
        }
    }

    fn handle_home_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('/') | KeyCode::Enter if self.input_mode == InputMode::Normal => {
                self.input_mode = InputMode::Editing;
            }
            KeyCode::Char('1') => self.mode = "single".to_string(),
            KeyCode::Char('2') => self.mode = "deep".to_string(),
            KeyCode::Char('3') => self.mode = "passive".to_string(),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.show_advanced = !self.show_advanced;
            }
            KeyCode::Char(' ') => {
                let installed: Vec<String> = self.tool_statuses.iter()
                    .filter(|t| t.installed && t.enabled).map(|t| t.name.clone()).collect();
                if self.selected_tools.len() == installed.len() {
                    self.selected_tools.clear();
                } else {
                    self.selected_tools = installed;
                }
            }
            _ => {}
        }
    }

    fn handle_scan_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('t') | KeyCode::Tab => {
                self.tab_selected = (self.tab_selected + 1) % 2;
            }
            KeyCode::Up => {
                self.auto_scroll = false;
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = self.scan_stream.len().saturating_sub(1);
                if self.scroll_offset < max {
                    self.scroll_offset += 1;
                }
                if self.scroll_offset >= max.saturating_sub(1) {
                    self.auto_scroll = true;
                }
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
                self.auto_scroll = false;
            }
            KeyCode::PageDown => {
                self.scroll_offset = (self.scroll_offset + 20)
                    .min(self.scan_stream.len().saturating_sub(1));
            }
            KeyCode::Home => { self.scroll_offset = 0; self.auto_scroll = false; }
            KeyCode::End => {
                self.scroll_offset = self.scan_stream.len().saturating_sub(1);
                self.auto_scroll = true;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.auto_scroll = true;
                self.scroll_offset = self.scan_stream.len().saturating_sub(1);
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if !self.scan_running && self.scan_session_id.is_some() {
                    self.export_report();
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if self.scan_running {
                    stop_scan(&self.state);
                    self.scan_running = false;
                }
            }
            _ => {}
        }
    }

    fn handle_findings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if self.active_finding.is_some() {
                    self.active_finding = None;
                }
            }
            KeyCode::Esc => {
                self.active_finding = None;
            }
            _ => {}
        }
    }

    fn handle_history_key(&mut self, _key: KeyEvent) {
        // placeholder for future interactive history
    }

    fn handle_tools_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('/') {
            self.input_mode = InputMode::Editing;
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => self.do_save_settings(),
            KeyCode::Char('1') => {
                let profiles = ["strong", "moderate", "off"];
                let current = profiles.iter()
                    .position(|&p| p == self.settings.sandbox_profile).unwrap_or(0);
                self.settings.sandbox_profile = profiles[(current + 1) % 3].to_string();
            }
            KeyCode::Char('2') => {
                self.settings.max_runtime = match self.settings.max_runtime {
                    300 => 600, 600 => 900, 900 => 1800, 1800 => 3600, _ => 300,
                };
            }
            _ => {}
        }
    }

    fn launch_scan(&mut self) {
        if self.target_input.trim().is_empty() {
            self.launch_error = "Target required.".to_string();
            return;
        }
        self.launch_error.clear();

        let tools = if self.selected_tools.is_empty() {
            vec![]
        } else {
            self.selected_tools.clone()
        };

        let request = ScanRequest {
            target: self.target_input.trim().to_string(),
            mode: self.mode.clone(),
            tools,
            report_template: if self.report_template_input.is_empty() { None } else { Some(self.report_template_input.clone()) },
            report_output_path: if self.report_path_input.is_empty() { None } else { Some(self.report_path_input.clone()) },
        };

        match start_scan(request, self.bus.clone(), Arc::clone(&self.state)) {
            Ok(session_id) => {
                self.scan_running = true;
                self.scan_complete = false;
                self.scan_session_id = Some(session_id);
                self.scan_stream.clear();
                self.scan_findings.clear();
                self.generated_report_path = None;
                self.scroll_offset = 0;
                self.auto_scroll = true;
                self.view = View::Scan;
            }
            Err(e) => self.launch_error = e,
        }
    }

    fn export_report(&mut self) {
        let sid = self.scan_session_id.clone().unwrap_or_default();
        if sid.is_empty() { return; }
        let request = ReportRequest {
            session_id: sid,
            template: if self.report_template_input.is_empty() { None } else { Some(self.report_template_input.clone()) },
            output_path: if self.report_path_input.is_empty() { None } else { Some(self.report_path_input.clone()) },
        };
        match generate_report(request, &self.state) {
            Ok(result) => self.generated_report_path = Some(result.path),
            Err(e) => self.settings.message = format!("Report error: {e}"),
        }
    }

    fn do_save_settings(&mut self) {
        let config = AppConfig {
            version: crate::core::types::TOROT_VERSION.to_string(),
            install_mode: self.settings.install_mode.clone(),
            default_report_template: self.settings.report_template.clone(),
            sandbox: SandboxConfig {
                profile: self.settings.sandbox_profile.clone(),
                max_runtime_seconds: self.settings.max_runtime,
                allow_network: true,
                writable_reports_only: true,
            },
            tools: vec![],
            knowledge_topics: vec![],
        };
        match save_settings(config, &self.state) {
            Ok(_) => {
                self.settings.saved = true;
                self.settings.message = "Settings saved.".to_string();
            }
            Err(e) => self.settings.message = format!("Save error: {e}"),
        }
    }

    fn render(&self, f: &mut Frame) {
        let size = f.area();
        if size.width < 60 || size.height < 20 {
            f.render_widget(
                Paragraph::new("Terminal too small. Resize to at least 60x20.")
                    .style(Style::new().fg(theme::RED)),
                size,
            );
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
            .split(size);

        self.render_titlebar(f, chunks[0]);
        self.render_body(f, chunks[1]);
        self.render_statusbar(f, chunks[2]);
    }

    fn render_titlebar(&self, f: &mut Frame, area: Rect) {
        let stats = &self.db_stats;
        let ss = stats.get("sessions").and_then(|v| v.as_i64()).unwrap_or(0);
        let sf = stats.get("findings").and_then(|v| v.as_i64()).unwrap_or(0);
        let sc = stats.get("critical").and_then(|v| v.as_i64()).unwrap_or(0);
        let left = format!(" TOROT v{} ", crate::core::types::TOROT_VERSION);
        let mid = format!("  sessions:{}  findings:{}", ss, sf);
        let right = if sc > 0 { format!("  CRITICAL:{}", sc) } else { String::new() };
        let title = format!("{}{}{}", left, mid, right);
        f.render_widget(
            Paragraph::new(title)
                .style(theme::titlebar_style()),
            area,
        );
    }

    fn render_statusbar(&self, f: &mut Frame, area: Rect) {
        let mode = if self.input_mode == InputMode::Editing { "EDIT" } else { " NORMAL " };
        let scan = if self.scan_running { " SCAN RUNNING " } else { "" };
        let left = format!(" {} | {} ", self.view.name(), mode);
        let right = format!("{}  q:quit  tab:nav", scan);
        let padding = area.width.saturating_sub((left.len() + right.len() + 2) as u16);
        let mid = " ".repeat(padding.saturating_sub(1) as usize);
        let content = format!("{}{}{}", left, mid, right);
        f.render_widget(
            Paragraph::new(content).style(theme::statusbar_style()),
            area,
        );
    }

    fn render_body(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(12), Constraint::Min(1)])
            .split(area);

        self.render_sidebar(f, chunks[0]);
        match self.view {
            View::Home => self.render_home_view(f, chunks[1]),
            View::Scan => self.render_scan_view(f, chunks[1]),
            View::Findings => self.render_findings_view(f, chunks[1]),
            View::History => self.render_history_view(f, chunks[1]),
            View::Tools => self.render_tools_view(f, chunks[1]),
            View::Settings => self.render_settings_view(f, chunks[1]),
        }
    }

    fn render_sidebar(&self, f: &mut Frame, area: Rect) {
        let items = [("1", "HOME"), ("2", "SCAN"), ("3", "FIND"), ("4", "HIST"), ("5", "TOOL"), ("6", "SET")];
        let mut lines: Vec<Line> = items
            .iter()
            .map(|(k, lbl)| {
                let active = self.view.name() == *lbl;
                Line::from(Span::styled(
                    format!(" {k} {lbl}"),
                    theme::sidebar_style(active),
                ))
            })
            .collect();

        let installed = self.tool_statuses.iter().filter(|t| t.installed).count();
        let total = self.tool_statuses.len();
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {installed}/{total}"),
            Style::new().fg(theme::FG_DIM),
        )));
        lines.push(Line::from(Span::styled(" tools", Style::new().fg(theme::FG_DIM))));

        f.render_widget(
            Paragraph::new(ratatui::text::Text::from(lines))
                .block(Block::default().borders(Borders::RIGHT).border_style(Style::new().fg(theme::BORDER))),
            area,
        );
    }

    fn render_home_view(&self, f: &mut Frame, area: Rect) {
        render_home(
            f, area,
            &self.target_input, &self.mode, &self.selected_tools,
            &self.tool_statuses, self.show_advanced,
            &self.report_path_input, &self.report_template_input,
            &self.launch_error, self.input_mode == InputMode::Editing,
        );
    }

    fn render_scan_view(&self, f: &mut Frame, area: Rect) {
        let s = ScanState {
            stream_lines: self.scan_stream.clone(),
            findings: self.scan_findings.clone(),
            running: self.scan_running,
            complete: self.scan_complete,
            target: self.target_input.clone(),
            scroll_offset: self.scroll_offset,
            auto_scroll: self.auto_scroll,
            tab_selected: self.tab_selected,
            generated_report_path: self.generated_report_path.clone(),
        };
        render_scan(f, area, &s);
    }

    fn render_findings_view(&self, f: &mut Frame, area: Rect) {
        let detail = self.active_finding.as_ref();
        render_findings(f, area, &self.findings, detail);
    }

    fn render_history_view(&self, f: &mut Frame, area: Rect) {
        render_history(f, area, &self.sessions);
    }

    fn render_tools_view(&self, f: &mut Frame, area: Rect) {
        render_tools(f, area, &self.tool_statuses, &self.tool_search);
    }

    fn render_settings_view(&self, f: &mut Frame, area: Rect) {
        render_settings(f, area, &self.settings);
    }
}
