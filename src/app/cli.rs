use crate::core::config::load_config;
use crate::core::engine::run_pipeline_cli;
use crate::core::report::render_report;
use crate::core::state::{get_findings_internal, load_session_from_db, AppState};
use crate::core::tools::{tool_statuses};
use crate::core::types::ScanRequest;
use anyhow::Result;
use std::sync::Arc;
use std::fs;

fn cli_usage() -> &'static str {
    "torot 4.0.0

Usage:
  torot tools                              List detected tools
  torot scan --target <value> [options]    Run a scan
  torot report --session <id> [options]    Generate a report
  torot config                             Show current config
  torot help                               Show this help"
}

fn cli_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|item| item == flag)
        .and_then(|idx| args.get(idx + 1).cloned())
}

fn cli_tools(args: &[String]) -> Vec<String> {
    cli_arg(args, "--tools")
        .unwrap_or_default()
        .split(',')
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_string())
        .collect()
}

pub fn run() -> Result<bool> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    if args.is_empty() || args.first().map(|s| s.as_str()) == Some("help") {
        println!("{}", cli_usage());
        return Ok(false);
    }

    let command = args.first().cloned().unwrap();
    if !matches!(command.as_str(), "tools" | "scan" | "report" | "config") {
        return Ok(false);
    }

    let state = Arc::new(AppState::new()?);
    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async move {
        match command.as_str() {
            "tools" => cmd_tools(&state).await?,
            "scan" => cmd_scan(&args, &state).await?,
            "report" => cmd_report(&args, &state).await?,
            "config" => cmd_config(&state).await?,
            _ => println!("{}", cli_usage()),
        }
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(true)
}

async fn cmd_tools(state: &Arc<AppState>) -> Result<()> {
    let config = load_config(state)?;
    let statuses = tool_statuses(&config);
    println!("{:<12} {:<9} {:<20} {}", "Tool", "Status", "Version", "Binary");
    println!("{}", "-".repeat(60));
    for tool in statuses {
        let status = if tool.installed { "installed" } else { "missing" };
        println!("{:<12} {:<9} {:<20} {}", tool.name, status, tool.version, tool.binary);
    }
    Ok(())
}

async fn cmd_scan(args: &[String], state: &Arc<AppState>) -> Result<()> {
    let target = cli_arg(args, "--target").ok_or_else(|| anyhow::anyhow!("Missing --target"))?;
    let tools = cli_tools(args);
    let template = cli_arg(args, "--template-file").and_then(|path| fs::read_to_string(path).ok());
    let mode = cli_arg(args, "--mode").unwrap_or_else(|| "single".to_string());

    let request = ScanRequest {
        target,
        mode,
        tools,
        report_template: template,
        report_output_path: cli_arg(args, "--output"),
    };

    let config = load_config(state)?;
    let _ = run_pipeline_cli(Arc::clone(state), request, config).await?;
    Ok(())
}

async fn cmd_report(args: &[String], state: &Arc<AppState>) -> Result<()> {
    let session_id = cli_arg(args, "--session").ok_or_else(|| anyhow::anyhow!("Missing --session"))?;
    let findings = get_findings_internal(&session_id, state);
    let session = load_session_from_db(state, &session_id)?
        .ok_or_else(|| anyhow::anyhow!("Session not found."))?;
    let config = load_config(state)?;

    let template = cli_arg(args, "--template-file")
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or(config.default_report_template);

    let body = render_report(&template, &session, &findings);
    let output = cli_arg(args, "--output").unwrap_or_else(|| {
        state
            .reports_dir
            .join(format!("{session_id}.md"))
            .to_string_lossy()
            .to_string()
    });

    fs::write(&output, &body)?;
    println!("Report written to {output}");
    Ok(())
}

async fn cmd_config(state: &Arc<AppState>) -> Result<()> {
    let config = load_config(state)?;
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}
