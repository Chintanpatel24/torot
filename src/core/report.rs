use crate::core::config::load_config;
use crate::core::parser::{render_findings_table, render_tool_overview, summarize_findings};
use crate::core::state::{get_findings_internal, load_session_from_db, AppState};
use crate::core::types::{Finding, ReportRequest, ReportResult, Session};
use crate::util::time::now_unix;
use std::fs;

pub fn render_report(template: &str, session: &Session, findings: &[Finding]) -> String {
    let summary = summarize_findings(findings);
    let critical = findings.iter().filter(|f| f.severity == "CRITICAL").count();
    let high = findings.iter().filter(|f| f.severity == "HIGH").count();
    let created_at = now_unix().to_string();

    template
        .replace("{{session_id}}", &session.id)
        .replace("{{target}}", &session.target)
        .replace("{{created_at}}", &created_at)
        .replace("{{findings_total}}", &findings.len().to_string())
        .replace("{{critical_count}}", &critical.to_string())
        .replace("{{high_count}}", &high.to_string())
        .replace("{{summary}}", &summary)
        .replace("{{tool_overview}}", &render_tool_overview(findings))
        .replace("{{findings_table}}", &render_findings_table(findings))
}

pub fn generate_report(request: ReportRequest, state: &AppState) -> Result<ReportResult, String> {
    let config = load_config(state).map_err(|e| e.to_string())?;
    let session = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&request.session_id).cloned()
    }
    .or_else(|| load_session_from_db(state, &request.session_id).ok().flatten())
    .ok_or_else(|| "Session not found.".to_string())?;

    let findings = get_findings_internal(&request.session_id, state);
    let template = request
        .template
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(config.default_report_template);

    let markdown = render_report(&template, &session, &findings);
    let path = request
        .output_path
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| {
            state
                .reports_dir
                .join(format!("{}.md", session.id))
                .to_string_lossy()
                .to_string()
        });

    fs::write(&path, &markdown).map_err(|e| e.to_string())?;
    Ok(ReportResult {
        session_id: session.id,
        path,
        summary: summarize_findings(&findings),
    })
}

pub fn generate_report_string(template: &str, session: &Session, findings: &[Finding]) -> String {
    render_report(template, session, findings)
}
