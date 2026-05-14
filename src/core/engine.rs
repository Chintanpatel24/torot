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

    let mut reader_handles: Vec<JoinHandle<Vec<String>>> = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        let bc = bus.clone();
        let _sid = session_id.to_string();
        let tool = profile.name.clone();
        reader_handles.push(tokio::spawn(async move { stream_reader(stdout, bc, tool).await }));
    }
    if let Some(stderr) = child.stderr.take() {
        let bc = bus.clone();
        let _sid = session_id.to_string();
        let tool = profile.name.clone();
        reader_handles.push(tokio::spawn(async move { stream_reader(stderr, bc, tool).await }));
    }

    let wait_result = tokio::time::timeout(Duration::from_secs(max_runtime_seconds), child.wait()).await;
    if wait_result.is_err() {
        let _ = child.kill().await;
        bus.emit(AppEvent::Line {
            tool: profile.name.clone(),
            line: format!("Timed out after {max_runtime_seconds} seconds."),
            kind: "system".to_string(),
            severity: Some("HIGH".to_string()),
        });
    }

    let mut output_lines = Vec::new();
    for handle in reader_handles {
        if let Ok(lines) = handle.await {
            output_lines.extend(lines);
        }
    }

    let combined = output_lines.join("\n");
    let findings = parse_output(session_id, profile, &combined);

    for finding in &findings {
        db::insert_finding(&state.db.lock().unwrap(), finding);
        if let Some(session) = state.sessions.lock().unwrap().get_mut(session_id) {
            session.findings.push(finding.clone());
        }
        bus.emit(AppEvent::Finding(finding.clone()));
    }

    bus.emit(AppEvent::Line {
        tool: profile.name.clone(),
        line: format!("{} complete with {} parsed finding(s).", profile.name, findings.len()),
        kind: "system".to_string(),
        severity: None,
    });
}

async fn stream_reader<R: tokio::io::AsyncRead + Unpin>(reader: R, bus: EventBus, tool: String) -> Vec<String> {
    let mut lines = BufReader::new(reader).lines();
    let mut output = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        bus.emit(AppEvent::Line {
            tool: tool.clone(),
            line: line.clone(),
            kind: "output".to_string(),
            severity: None,
        });
        output.push(line);
    }
    output
}

async fn run_tool_cli(session_id: &str, target: &str, profile: ToolProfile, reports_dir: std::path::PathBuf) -> Vec<Finding> {
    let runtime = detect_tool(&profile);
    let Some(binary) = runtime.binary else {
        eprintln!("{} missing: {}", profile.name, profile.install_hint);
        return Vec::new();
    };
    let report_file = reports_dir.join(format!("{}-{}.out", session_id, profile.name));
    let Some(args) = render_args(&profile, target, &report_file) else {
        eprintln!("{} skipped: incompatible target type", profile.name);
        return Vec::new();
    };

    println!("[torot] {} {}", profile.name, args.join(" "));

    let output = tokio::time::timeout(
        Duration::from_secs(profile.timeout_seconds.max(30)),
        TokioCommand::new(&binary).args(&args).output(),
    )
    .await;

    match output {
        Ok(Ok(result)) => {
            let mut text = String::from_utf8_lossy(&result.stdout).to_string();
            if !result.stderr.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&String::from_utf8_lossy(&result.stderr));
            }
            parse_output(session_id, &profile, &text)
        }
        Ok(Err(err)) => {
            eprintln!("{} failed: {err}", profile.name);
            Vec::new()
        }
        Err(_) => {
            eprintln!("{} timed out", profile.name);
            Vec::new()
        }
    }
}

pub async fn run_pipeline_cli(
    state: Arc<AppState>,
    request: ScanRequest,
    config: crate::core::types::AppConfig,
) -> anyhow::Result<String> {
    let session = Session::new(&request.target, &request.mode);
    let session_id = session.id.clone();
    state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id.clone(), session.clone());

    db::insert_session(
        &state.db.lock().unwrap(),
        &session_id,
        &request.target,
        crate::core::tools::infer_target_kind(&request.target),
        session.start_time,
    );

    let selected = if request.tools.is_empty() {
        suggest_tools(&config, &request.target)
    } else {
        request.tools.clone()
    };

    let tools_by_name: HashMap<String, ToolProfile> = config
        .tools
        .iter()
        .cloned()
        .map(|tool| (tool.name.clone(), tool))
        .collect();

    let mut handles: Vec<JoinHandle<Vec<Finding>>> = Vec::new();
    for tool_name in selected {
        if let Some(profile) = tools_by_name.get(&tool_name).cloned() {
            let target = request.target.clone();
            let reports_dir = state.reports_dir.clone();
            let sid = session_id.clone();
            handles.push(tokio::spawn(async move {
                run_tool_cli(&sid, &target, profile, reports_dir).await
            }));
        }
    }

    let mut findings = Vec::new();
    for handle in handles {
        if let Ok(tool_findings) = handle.await {
            findings.extend(tool_findings);
        }
    }

    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(current) = sessions.get_mut(&session_id) {
            current.findings = findings.clone();
        }
    }

    {
        let db = state.db.lock().unwrap();
        for finding in &findings {
            db::insert_finding(&db, finding);
        }
        db::update_session(&db, &session_id, now_unix(), findings.len() as u32, &summarize_findings(&findings));
    }

    let template = request
        .report_template
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(config.default_report_template.clone());

    let report_path = request
        .report_output_path
        .clone()
        .unwrap_or_else(|| state.reports_dir.join(format!("{session_id}.md")).to_string_lossy().to_string());

    let markdown = render_report(&template, &session, &findings);
    fs::write(&report_path, &markdown)?;
    println!("[torot] report written to {report_path}");
    println!("[torot] {}", summarize_findings(&findings));
    Ok(session_id)
}
