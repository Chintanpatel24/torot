use crate::core::config::load_config;
use crate::core::db;
use crate::core::event::{AppEvent, EventBus};
use crate::core::parser::parse_output;
use crate::core::state::{get_findings_internal, AppState};
use crate::core::tools::{detect_tool, render_args, suggest_tools};
use crate::core::types::{Finding, ScanRequest, Session, ToolProfile, TOROT_VERSION};
use crate::core::report::render_report;
use crate::core::parser::summarize_findings;
use crate::util::time::now_unix;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::task::JoinHandle;

pub fn start_scan(request: ScanRequest, bus: EventBus, state: Arc<AppState>) -> Result<String, String> {
    let config = load_config(&state).map_err(|e| e.to_string())?;
    let mut session = Session::new(&request.target, &request.mode);
    let session_id = session.id.clone();
    session.report_path = request.report_output_path.clone();

    {
        state.sessions.lock().unwrap().insert(session_id.clone(), session.clone());
        *state.active_scan.lock().unwrap() = Some(session_id.clone());
    }

    db::insert_session(
        &state.db.lock().unwrap(),
        &session_id,
        &request.target,
        crate::core::tools::infer_target_kind(&request.target),
        session.start_time,
    );

    bus.emit(AppEvent::Line {
        tool: "torot".to_string(),
        line: format!("torot v{} starting {} scan against {}", TOROT_VERSION, request.mode, request.target),
        kind: "system".to_string(),
        severity: None,
    });

    let selected_tools = if request.tools.is_empty() {
        suggest_tools(&config, &request.target)
    } else {
        request.tools.clone()
    };

    tokio::spawn(async move {
        run_pipeline(session_id, request, selected_tools, config, bus, state).await;
    });

    Ok(session.id)
}

async fn run_pipeline(
    session_id: String,
    request: ScanRequest,
    selected_tools: Vec<String>,
    config: crate::core::types::AppConfig,
    bus: EventBus,
    state: Arc<AppState>,
) {
    if selected_tools.is_empty() {
        bus.emit(AppEvent::Line {
            tool: "torot".to_string(),
            line: "No compatible installed tools were found for this target.".to_string(),
            kind: "system".to_string(),
            severity: Some("HIGH".to_string()),
        });
        bus.emit(AppEvent::ScanComplete { report_path: None });
        return;
    }

    bus.emit(AppEvent::Line {
        tool: "torot".to_string(),
        line: format!("Launching {} tool(s) in parallel: {}", selected_tools.len(), selected_tools.join(", ")),
        kind: "system".to_string(),
        severity: None,
    });

    let tools_by_name: HashMap<String, ToolProfile> = config
        .tools
        .iter()
        .cloned()
        .map(|tool| (tool.name.clone(), tool))
        .collect();

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    for name in selected_tools {
        let Some(profile) = tools_by_name.get(&name).cloned() else {
            bus.emit(AppEvent::Line {
                tool: "torot".to_string(),
                line: format!("Unknown tool `{name}` skipped."),
                kind: "system".to_string(),
                severity: Some("MEDIUM".to_string()),
            });
            continue;
        };

        let bus_clone = bus.clone();
        let state_clone = Arc::clone(&state);
        let session_clone = session_id.clone();
        let target_clone = request.target.clone();
        let max_runtime = config.sandbox.max_runtime_seconds.min(profile.timeout_seconds).max(30);

        tasks.push(tokio::spawn(async move {
            run_tool(&session_clone, &target_clone, &profile, max_runtime, bus_clone, state_clone).await;
        }));
    }

    for task in tasks {
        let _ = task.await;
    }

    finish_pipeline(session_id, request, config, bus, state).await;
}

async fn finish_pipeline(
    session_id: String,
    request: ScanRequest,
    config: crate::core::types::AppConfig,
    bus: EventBus,
    state: Arc<AppState>,
) {
    let findings = get_findings_internal(&session_id, &state);
    let summary = summarize_findings(&findings);
    let report_template = request
        .report_template
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(config.default_report_template.clone());

    let report_path = {
        let session = {
            let sessions = state.sessions.lock().unwrap();
            sessions.get(&session_id).cloned()
        };
        match session {
            Some(session) => {
                let path = request.report_output_path.clone().unwrap_or_else(|| {
                    state.reports_dir.join(format!("{}.md", session.id)).to_string_lossy().to_string()
                });
                let markdown = render_report(&report_template, &session, &findings);
                if fs::write(&path, &markdown).is_ok() {
                    let mut sessions = state.sessions.lock().unwrap();
                    if let Some(active) = sessions.get_mut(&session_id) {
                        active.report_path = Some(path.clone());
                    }
                    Some(path)
                } else {
                    None
                }
            }
            None => None,
        }
    };

    let end_time = now_unix();
    db::update_session(&state.db.lock().unwrap(), &session_id, end_time, findings.len() as u32, &summary);

    bus.emit(AppEvent::Line {
        tool: "torot".to_string(),
        line: format!(
            "Scan complete. {}{}",
            summarize_findings(&findings),
            report_path.as_ref().map(|p| format!(" Report: {p}")).unwrap_or_default()
        ),
        kind: "system".to_string(),
        severity: None,
    });
    bus.emit(AppEvent::ScanComplete { report_path });
}

async fn run_tool(
    session_id: &str,
    target: &str,
    profile: &ToolProfile,
    max_runtime_seconds: u64,
    bus: EventBus,
    state: Arc<AppState>,
) {
    let runtime = detect_tool(profile);
    let Some(binary) = runtime.binary else {
        bus.emit(AppEvent::Line {
            tool: profile.name.clone(),
            line: format!("{} not installed. {}", profile.name, profile.install_hint),
            kind: "system".to_string(),
            severity: Some("MEDIUM".to_string()),
        });
        return;
    };

    let report_file = state.reports_dir.join(format!("{}-{}.out", session_id, profile.name));
    let Some(args) = render_args(profile, target, &report_file) else {
        bus.emit(AppEvent::Line {
            tool: profile.name.clone(),
            line: "Target type does not match this tool's supported inputs.".to_string(),
            kind: "system".to_string(),
            severity: Some("LOW".to_string()),
        });
        return;
    };

    bus.emit(AppEvent::Line {
        tool: profile.name.clone(),
        line: format!("Starting {} with {}", profile.name, args.join(" ")),
        kind: "system".to_string(),
        severity: None,
    });

    let mut command = TokioCommand::new(&binary);
    command.args(&args);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    command.env_clear();
    command.env("PATH", std::env::var("PATH").unwrap_or_default());
    command.env("HOME", std::env::var("HOME").unwrap_or_default());
    command.env("TOROT_SANDBOX_PROFILE", "strong");
    command.env("TOROT_ALLOWED_TARGET", target);
    command.current_dir(if Path::new(target).is_dir() { target } else { "." });

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            bus.emit(AppEvent::Line {
                tool: profile.name.clone(),
                line: format!("Launch failed: {err}"),
                kind: "system".to_string(),
                severity: Some("HIGH".to_string()),
            });
            return;
        }
    };

