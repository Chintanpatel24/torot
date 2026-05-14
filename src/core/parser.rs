use crate::core::types::{Finding, ToolProfile};
use std::collections::{HashMap, HashSet};

pub fn severity_rank(value: &str) -> u8 {
    match value {
        "CRITICAL" => 0,
        "HIGH" => 1,
        "MEDIUM" => 2,
        "LOW" => 3,
        _ => 4,
    }
}

pub fn severity_from_text(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if lower.contains("critical") {
        "CRITICAL"
    } else if lower.contains(" high") || lower.starts_with("high") || lower.contains("error") || lower.contains("rce") {
        "HIGH"
    } else if lower.contains("medium") || lower.contains("warning") || lower.contains("warn") {
        "MEDIUM"
    } else if lower.contains("low") {
        "LOW"
    } else {
        "INFO"
    }
}

pub fn parse_output(session_id: &str, profile: &ToolProfile, output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        findings.extend(parse_json(session_id, profile, &json));
    }

    if findings.is_empty() {
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    findings.extend(parse_json(session_id, profile, &json));
                }
            }
        }
    }

    if findings.is_empty() {
        findings.extend(parse_text(session_id, profile, output));
    }

    findings.sort_by(|a, b| severity_rank(&a.severity).cmp(&severity_rank(&b.severity)));
    findings.dedup_by(|a, b| a.tool == b.tool && a.title == b.title && a.description == b.description);
    findings
}

fn parse_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value) -> Vec<Finding> {
    let tool = profile.name.as_str();
    let mut findings = Vec::new();

    match tool {
        "nuclei" => parse_nuclei_json(session_id, profile, value, &mut findings),
        "semgrep" => parse_semgrep_json(session_id, profile, value, &mut findings),
        "gitleaks" => parse_gitleaks_json(session_id, profile, value, &mut findings),
        "trufflehog" => parse_trufflehog_json(session_id, profile, value, &mut findings),
        "httpx" | "subfinder" | "amass" | "katana" => parse_discovery_json(session_id, profile, value, &mut findings),
        _ => {}
    }

    findings
}

fn parse_nuclei_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value, findings: &mut Vec<Finding>) {
    if let Some(info) = value.get("info") {
        let severity = info
            .get("severity")
            .and_then(|v| v.as_str())
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "INFO".to_string());
        let mut finding = Finding::new(
            session_id,
            profile.name.as_str(),
            &format!("[nuclei] {}", info.get("name").and_then(|v| v.as_str()).unwrap_or("template hit")),
            &severity,
        );
        finding.domain = profile.domain.clone();
        finding.description = info.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        finding.file = value.get("matched-at").and_then(|v| v.as_str()).unwrap_or("").to_string();
        findings.push(finding);
    }
}

fn parse_semgrep_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value, findings: &mut Vec<Finding>) {
    if let Some(results) = value.get("results").and_then(|v| v.as_array()) {
        for result in results {
            let severity = result
                .pointer("/extra/severity")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "ERROR" => "HIGH",
                    "WARNING" => "MEDIUM",
                    _ => "LOW",
                })
                .unwrap_or("INFO")
                .to_string();
            let check = result.get("check_id").and_then(|v| v.as_str()).unwrap_or("rule");
            let mut finding = Finding::new(session_id, profile.name.as_str(), &format!("[semgrep] {check}"), &severity);
            finding.domain = profile.domain.clone();
            finding.description = result.pointer("/extra/message").and_then(|v| v.as_str()).unwrap_or("").to_string();
            finding.file = result.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            finding.line = result.pointer("/start/line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            findings.push(finding);
        }
    }
}

fn parse_gitleaks_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value, findings: &mut Vec<Finding>) {
    if let Some(items) = value.as_array() {
        for item in items {
            let mut finding = Finding::new(session_id, profile.name.as_str(), "[gitleaks] secret exposure", "HIGH");
            finding.domain = profile.domain.clone();
            finding.description = item.get("Description").and_then(|v| v.as_str()).unwrap_or("Potential secret exposure").to_string();
            finding.file = item.get("File").and_then(|v| v.as_str()).unwrap_or("").to_string();
            finding.line = item.get("StartLine").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            finding.fix_suggestion = "Rotate the credential, remove it from source control, and move it to a secret manager.".to_string();
            findings.push(finding);
        }
    }
}

fn parse_trufflehog_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value, findings: &mut Vec<Finding>) {
    let verified = value.get("Verified").and_then(|v| v.as_bool()).unwrap_or(false);
    if value.get("SourceMetadata").is_some() || value.get("DetectorName").is_some() {
        let mut finding = Finding::new(
            session_id,
            profile.name.as_str(),
            "[trufflehog] possible secret exposure",
            if verified { "CRITICAL" } else { "HIGH" },
        );
        finding.domain = profile.domain.clone();
        finding.description = format!(
            "Detector: {}",
            value.get("DetectorName").and_then(|v| v.as_str()).unwrap_or("unknown")
        );
        findings.push(finding);
    }
}

fn parse_discovery_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value, findings: &mut Vec<Finding>) {
    if let Some(url) = value
        .get("url")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("host").and_then(|v| v.as_str()))
        .or_else(|| value.get("name").and_then(|v| v.as_str()))
    {
        let mut finding = Finding::new(session_id, profile.name.as_str(), &format!("[{}] discovery", profile.name), "INFO");
        finding.domain = profile.domain.clone();
        finding.description = url.to_string();
        findings.push(finding);
    }
}

fn parse_text(session_id: &str, profile: &ToolProfile, output: &str) -> Vec<Finding> {
    let keywords = [
        "critical", "high", "medium", "warning", "error", "exposed",
        "vulnerable", "sql injection", "xss", "ssrf", "takeover",
        "open port", "directory listing", "default credentials", "secret", "token",
    ];

    output
        .lines()
        .filter_map(|line| {
            let text = line.trim();
            if text.len() < 8 {
                return None;
            }
            let lower = text.to_lowercase();
            if !keywords.iter().any(|kw| lower.contains(kw)) {
                return None;
            }

            let mut finding = Finding::new(
                session_id,
                &profile.name,
                &format!("[{}] {}", profile.name, text.chars().take(80).collect::<String>()),
                severity_from_text(text),
            );
            finding.domain = profile.domain.clone();
            finding.description = text.to_string();

            if lower.contains("sql injection") {
                finding.fix_suggestion =
                    "Validate the injection manually and move the affected parameter to prepared statements."
                        .to_string();
                finding.impact = "Potential database read/write compromise.".to_string();
            }

            Some(finding)
        })
        .collect()
}

pub fn summarize_findings(findings: &[Finding]) -> String {
    let critical = findings.iter().filter(|f| f.severity == "CRITICAL").count();
    let high = findings.iter().filter(|f| f.severity == "HIGH").count();
    let medium = findings.iter().filter(|f| f.severity == "MEDIUM").count();
    let unique_tools = findings.iter().map(|f| f.tool.clone()).collect::<HashSet<_>>().len();

    if findings.is_empty() {
        "No findings were extracted from the selected tool outputs.".to_string()
    } else {
        format!(
            "{} finding(s) total across {} tool(s): {} critical, {} high, {} medium.",
            findings.len(),
            unique_tools,
            critical,
            high,
            medium
        )
    }
}

pub fn render_tool_overview(findings: &[Finding]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for finding in findings {
        *counts.entry(finding.tool.clone()).or_insert(0) += 1;
    }
    if counts.is_empty() {
        return "_No tool produced parsed findings._".to_string();
    }
    let mut rows = counts.into_iter().collect::<Vec<_>>();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows.into_iter()
        .map(|(tool, count)| format!("- `{}`: {} finding(s)", tool, count))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_findings_table(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "_No findings captured._".to_string();
    }
    let mut lines = vec![
        "| Severity | Tool | Title | Evidence |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
    ];
    for finding in findings {
        let evidence = if !finding.file.is_empty() {
            if finding.line > 0 {
                format!("{}:{}", finding.file, finding.line)
            } else {
                finding.file.clone()
            }
        } else {
            finding.description.chars().take(48).collect()
        };
        lines.push(format!(
            "| {} | {} | {} | {} |",
            finding.severity,
            finding.tool,
            finding.title.replace('|', "/"),
            evidence.replace('|', "/"),
        ));
    }
    lines.join("\n")
}
