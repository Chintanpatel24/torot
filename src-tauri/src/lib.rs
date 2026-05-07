use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::task::JoinHandle;
use uuid::Uuid;

const TOROT_VERSION: &str = "4.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub session_id: String,
    pub tool: String,
    pub title: String,
    pub severity: String,
    pub domain: String,
    pub description: String,
    pub file: String,
    pub line: u32,
    pub code_snippet: String,
    pub fix_suggestion: String,
    pub impact: String,
    pub bug_type: String,
    pub timestamp: u64,
}

impl Finding {
    pub fn new(session_id: &str, tool: &str, title: &str, severity: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            tool: tool.to_string(),
            title: title.to_string(),
            severity: severity.to_string(),
            domain: String::new(),
            description: String::new(),
            file: String::new(),
            line: 0,
            code_snippet: String::new(),
            fix_suggestion: String::new(),
            impact: String::new(),
            bug_type: String::new(),
            timestamp: now_unix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub target: String,
    pub mode: String,
    pub start_time: u64,
    pub findings: Vec<Finding>,
    pub report_path: Option<String>,
}

impl Session {
    pub fn new(target: &str, mode: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..12].to_string(),
            target: target.to_string(),
            mode: mode.to_string(),
            start_time: now_unix(),
            findings: Vec::new(),
            report_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSession {
    pub id: String,
    pub target: String,
    pub domain: String,
    pub start_time: u64,
    pub end_time: u64,
    pub total_findings: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamLine {
    pub session_id: String,
    pub tool: String,
    pub line: String,
    pub kind: String,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardStep {
    pub order: u8,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProfile {
    pub name: String,
    pub domain: String,
    pub description: String,
    pub binary_names: Vec<String>,
    pub path_override: Option<String>,
    pub args: Vec<String>,
    pub version_args: Vec<String>,
    pub install_hint: String,
    pub output_format: String,
    pub input_kinds: Vec<String>,
    pub source: String,
    pub auto_detect: bool,
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub capabilities: Vec<String>,
    pub knowledge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub name: String,
    pub installed: bool,
    pub binary: String,
    pub version: String,
    pub domain: String,
    pub description: String,
    pub install_hint: String,
    pub output_format: String,
    pub source: String,
    pub auto_detect: bool,
    pub enabled: bool,
    pub capabilities: Vec<String>,
    pub knowledge: Vec<String>,
    pub wizard_steps: Vec<WizardStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub profile: String,
    pub max_runtime_seconds: u64,
    pub allow_network: bool,
    pub writable_reports_only: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            profile: "strong".to_string(),
            max_runtime_seconds: 900,
            allow_network: true,
            writable_reports_only: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub install_mode: String,
    pub default_report_template: String,
    pub sandbox: SandboxConfig,
    pub tools: Vec<ToolProfile>,
    pub knowledge_topics: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: TOROT_VERSION.to_string(),
            install_mode: "both".to_string(),
            default_report_template: default_report_template(),
            sandbox: SandboxConfig::default(),
            tools: builtin_tools(),
            knowledge_topics: builtin_knowledge_topics(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub install_mode: String,
    pub cli_supported: bool,
    pub knowledge_topics: Vec<String>,
    pub report_template_placeholders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProfileInput {
    pub name: String,
    pub domain: String,
    pub description: String,
    pub binary_names: Vec<String>,
    pub path_override: Option<String>,
    pub args: Vec<String>,
    pub version_args: Vec<String>,
    pub install_hint: String,
    pub output_format: String,
    pub input_kinds: Vec<String>,
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub capabilities: Vec<String>,
    pub knowledge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    pub target: String,
    pub mode: String,
    pub tools: Vec<String>,
    pub report_template: Option<String>,
    pub report_output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub session_id: String,
    pub template: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResult {
    pub session_id: String,
    pub path: String,
    pub summary: String,
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub sessions: Mutex<HashMap<String, Session>>,
    pub active_scan: Mutex<Option<String>>,
    pub config_path: PathBuf,
    pub reports_dir: PathBuf,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let dir = data_dir();
        let reports_dir = dir.join("reports");
        fs::create_dir_all(&reports_dir)?;
        let conn = Connection::open(dir.join("memory.db"))?;
        init_schema(&conn)?;
        let config_path = dir.join("config.json");
        ensure_config_file(&config_path)?;
        Ok(Self {
            db: Mutex::new(conn),
            sessions: Mutex::new(HashMap::new()),
            active_scan: Mutex::new(None),
            config_path,
            reports_dir,
        })
    }
}

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".torot")
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            target TEXT,
            domain TEXT,
            start_time INTEGER,
            end_time INTEGER,
            total_findings INTEGER DEFAULT 0,
            summary TEXT
        );
        CREATE TABLE IF NOT EXISTS findings (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            tool TEXT,
            title TEXT,
            severity TEXT,
            domain TEXT,
            description TEXT,
            file TEXT,
            line INTEGER,
            code_snippet TEXT,
            fix_suggestion TEXT,
            impact TEXT,
            bug_type TEXT,
            timestamp INTEGER
        );
    ",
    )?;
    Ok(())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn default_report_template() -> String {
    [
        "# Torot v4 Report",
        "",
        "- Session: `{{session_id}}`",
        "- Target: `{{target}}`",
        "- Generated: `{{created_at}}`",
        "- Findings: `{{findings_total}}`",
        "- Critical: `{{critical_count}}`",
        "- High: `{{high_count}}`",
        "",
        "## Executive Summary",
        "{{summary}}",
        "",
        "## Tool Coverage",
        "{{tool_overview}}",
        "",
        "## Findings",
        "{{findings_table}}",
    ]
    .join("\n")
}

fn builtin_knowledge_topics() -> Vec<String> {
    vec![
        "attack-surface-mapping".to_string(),
        "subdomain-enumeration".to_string(),
        "web-application-testing".to_string(),
        "api-security".to_string(),
        "secrets-exposure".to_string(),
        "network-recon".to_string(),
        "sandbox-aware-execution".to_string(),
    ]
}

fn builtin_tools() -> Vec<ToolProfile> {
    vec![
        builtin_tool(
            "nmap",
            "webapp",
            "Host and service discovery for domains, IPs, and exposed ports.",
            &["nmap"],
            &["-sV", "-Pn", "{{target_host}}"],
            &["--version"],
            "Install nmap with your package manager.",
            "text",
            &["host", "url"],
            900,
            &["recon", "port-scan", "service-detection"],
            &["attack-surface-mapping", "network-recon"],
        ),
        builtin_tool(
            "bbot",
            "webapp",
            "Asset discovery and bug bounty reconnaissance automation.",
            &["bbot"],
            &["-t", "{{target_host}}", "-f", "subdomain-enum", "web-basic", "-y"],
            &["--version"],
            "Install with: pipx install bbot",
            "text",
            &["host", "url"],
            1200,
            &["recon", "subdomains", "web-enum"],
            &["subdomain-enumeration", "attack-surface-mapping"],
        ),
        builtin_tool(
            "nuclei",
            "webapp",
            "Template-based vulnerability scanning.",
            &["nuclei"],
            &["-target", "{{target_url}}", "-jsonl"],
            &["-version"],
            "Install with: go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest",
            "jsonl",
            &["url", "host"],
            900,
            &["vuln-scan", "templates"],
            &["web-application-testing"],
        ),
        builtin_tool(
            "httpx",
            "webapp",
            "HTTP probing and metadata discovery.",
            &["httpx"],
            &["-u", "{{target_url}}", "-json"],
            &["-version"],
            "Install with: go install github.com/projectdiscovery/httpx/cmd/httpx@latest",
            "jsonl",
            &["url", "host"],
            300,
            &["recon", "http-probing"],
            &["attack-surface-mapping"],
        ),
        builtin_tool(
            "subfinder",
            "webapp",
            "Passive subdomain enumeration.",
            &["subfinder"],
            &["-d", "{{target_host}}", "-silent", "-oJ"],
            &["-version"],
            "Install with: go install github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest",
            "jsonl",
            &["host", "url"],
            600,
            &["recon", "subdomains"],
            &["subdomain-enumeration"],
        ),
        builtin_tool(
            "amass",
            "webapp",
            "Enumeration and DNS intelligence.",
            &["amass"],
            &["enum", "-passive", "-d", "{{target_host}}", "-json", "-"],
            &["-version"],
            "Install with: go install github.com/owasp-amass/amass/v4/...@master",
            "json",
            &["host", "url"],
            1200,
            &["recon", "subdomains", "dns"],
            &["subdomain-enumeration"],
        ),
        builtin_tool(
            "katana",
            "webapp",
            "Web crawling and endpoint enumeration.",
            &["katana"],
            &["-u", "{{target_url}}", "-jsonl"],
            &["-version"],
            "Install with: go install github.com/projectdiscovery/katana/cmd/katana@latest",
            "jsonl",
            &["url"],
            900,
            &["crawl", "content-discovery"],
            &["attack-surface-mapping"],
        ),
        builtin_tool(
            "ffuf",
            "webapp",
            "Directory and parameter fuzzing.",
            &["ffuf"],
            &["-u", "{{target_url}}/FUZZ", "-w", "/usr/share/wordlists/dirb/common.txt", "-mc", "all"],
            &["-V"],
            "Install with: go install github.com/ffuf/ffuf/v2@latest",
            "text",
            &["url"],
            900,
            &["fuzz", "content-discovery"],
            &["web-application-testing"],
        ),
        builtin_tool(
            "gobuster",
            "webapp",
            "Directory brute forcing.",
            &["gobuster"],
            &["dir", "-u", "{{target_url}}", "-w", "/usr/share/wordlists/dirb/common.txt"],
            &["version"],
            "Install with: go install github.com/OJ/gobuster/v3@latest",
            "text",
            &["url"],
            900,
            &["fuzz", "content-discovery"],
            &["web-application-testing"],
        ),
        builtin_tool(
            "nikto",
            "webapp",
            "Baseline web server checks.",
            &["nikto"],
            &["-h", "{{target_url}}", "-Format", "txt"],
            &["-Version"],
            "Install nikto with your package manager.",
            "text",
            &["url"],
            900,
            &["vuln-scan", "web-baseline"],
            &["web-application-testing"],
        ),
        builtin_tool(
            "sqlmap",
            "api",
            "Automated SQL injection verification.",
            &["sqlmap"],
            &["-u", "{{target_url}}", "--batch", "--level", "2"],
            &["--version"],
            "Install with: pipx install sqlmap",
            "text",
            &["url"],
            1200,
            &["sqli", "verification"],
            &["api-security", "web-application-testing"],
        ),
        builtin_tool(
            "semgrep",
            "general",
            "Static analysis for code and config targets.",
            &["semgrep"],
            &["--config", "auto", "--json", "{{target}}"],
            &["--version"],
            "Install with: pipx install semgrep",
            "json",
            &["directory", "file"],
            900,
            &["static-analysis", "code-review"],
            &["web-application-testing", "api-security"],
        ),
        builtin_tool(
            "trufflehog",
            "general",
            "Secrets discovery in repositories and filesystems.",
            &["trufflehog"],
            &["filesystem", "{{target}}", "--json"],
            &["--version"],
            "Install with: go install github.com/trufflesecurity/trufflehog/v3@latest",
            "jsonl",
            &["directory", "file"],
            900,
            &["secrets", "leaks"],
            &["secrets-exposure"],
        ),
        builtin_tool(
            "gitleaks",
            "general",
            "High-signal secret scanning for repositories.",
            &["gitleaks"],
            &["detect", "--source", "{{target}}", "--report-format", "json"],
            &["version"],
            "Install with: go install github.com/zricethezav/gitleaks/v8@latest",
            "json",
            &["directory", "file"],
            900,
            &["secrets", "leaks"],
            &["secrets-exposure"],
        ),
    ]
}

fn builtin_tool(
    name: &str,
    domain: &str,
    description: &str,
    binaries: &[&str],
    args: &[&str],
    version_args: &[&str],
    install_hint: &str,
    output_format: &str,
    input_kinds: &[&str],
    timeout_seconds: u64,
    capabilities: &[&str],
    knowledge: &[&str],
) -> ToolProfile {
    ToolProfile {
        name: name.to_string(),
        domain: domain.to_string(),
        description: description.to_string(),
        binary_names: binaries.iter().map(|s| s.to_string()).collect(),
        path_override: None,
        args: args.iter().map(|s| s.to_string()).collect(),
        version_args: version_args.iter().map(|s| s.to_string()).collect(),
        install_hint: install_hint.to_string(),
        output_format: output_format.to_string(),
        input_kinds: input_kinds.iter().map(|s| s.to_string()).collect(),
        source: "builtin".to_string(),
        auto_detect: true,
        enabled: true,
        timeout_seconds,
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        knowledge: knowledge.iter().map(|s| s.to_string()).collect(),
    }
}

fn ensure_config_file(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let config = AppConfig::default();
    save_config_to_path(path, &config)
}

fn load_config_from_path(path: &Path) -> Result<AppConfig> {
    let raw = fs::read_to_string(path)?;
    let mut parsed: AppConfig = serde_json::from_str(&raw).unwrap_or_default();
    parsed.version = TOROT_VERSION.to_string();
    parsed.tools = merge_builtin_tools(parsed.tools);
    if parsed.default_report_template.trim().is_empty() {
        parsed.default_report_template = default_report_template();
    }
    if parsed.knowledge_topics.is_empty() {
        parsed.knowledge_topics = builtin_knowledge_topics();
    }
    Ok(parsed)
}

fn save_config_to_path(path: &Path, config: &AppConfig) -> Result<()> {
    let body = serde_json::to_string_pretty(config)?;
    fs::write(path, body)?;
    Ok(())
}

fn load_config(state: &AppState) -> Result<AppConfig> {
    load_config_from_path(&state.config_path)
}

fn save_config(state: &AppState, config: &AppConfig) -> Result<()> {
    save_config_to_path(&state.config_path, config)
}

fn merge_builtin_tools(existing: Vec<ToolProfile>) -> Vec<ToolProfile> {
    let mut merged = builtin_tools();
    let mut index: HashMap<String, usize> = merged
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), i))
        .collect();

    for tool in existing {
        if let Some(pos) = index.get(&tool.name).copied() {
            merged[pos].path_override = tool.path_override;
            merged[pos].args = if tool.args.is_empty() {
                merged[pos].args.clone()
            } else {
                tool.args
            };
            merged[pos].version_args = if tool.version_args.is_empty() {
                merged[pos].version_args.clone()
            } else {
                tool.version_args
            };
            merged[pos].install_hint = if tool.install_hint.trim().is_empty() {
                merged[pos].install_hint.clone()
            } else {
                tool.install_hint
            };
            merged[pos].output_format = if tool.output_format.trim().is_empty() {
                merged[pos].output_format.clone()
            } else {
                tool.output_format
            };
            merged[pos].input_kinds = if tool.input_kinds.is_empty() {
                merged[pos].input_kinds.clone()
            } else {
                tool.input_kinds
            };
            merged[pos].enabled = tool.enabled;
            merged[pos].timeout_seconds = tool.timeout_seconds.max(30);
            merged[pos].capabilities = if tool.capabilities.is_empty() {
                merged[pos].capabilities.clone()
            } else {
                tool.capabilities
            };
            merged[pos].knowledge = if tool.knowledge.is_empty() {
                merged[pos].knowledge.clone()
            } else {
                tool.knowledge
            };
        } else {
            let mut custom = tool.clone();
            custom.source = "custom".to_string();
            index.insert(custom.name.clone(), merged.len());
            merged.push(custom);
        }
    }

    merged.sort_by(|a, b| a.name.cmp(&b.name));
    merged
}

fn tool_statuses(config: &AppConfig) -> Vec<ToolStatus> {
    config
        .tools
        .iter()
        .map(|profile| {
            let runtime = detect_tool(profile);
            ToolStatus {
                name: profile.name.clone(),
                installed: runtime.installed,
                binary: runtime.binary.unwrap_or_default(),
                version: runtime.version.unwrap_or_default(),
                domain: profile.domain.clone(),
                description: profile.description.clone(),
                install_hint: profile.install_hint.clone(),
                output_format: profile.output_format.clone(),
                source: profile.source.clone(),
                auto_detect: profile.auto_detect,
                enabled: profile.enabled,
                capabilities: profile.capabilities.clone(),
                knowledge: profile.knowledge.clone(),
                wizard_steps: wizard_steps(profile, runtime.installed),
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ToolRuntime {
    installed: bool,
    binary: Option<String>,
    version: Option<String>,
}

fn detect_tool(profile: &ToolProfile) -> ToolRuntime {
    let candidate = profile
        .path_override
        .clone()
        .filter(|p| Path::new(p).exists())
        .or_else(|| {
            profile.binary_names.iter().find_map(|bin| {
                which::which(bin)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
        });

    let version = candidate
        .as_ref()
        .and_then(|binary| detect_version(binary, &profile.version_args));

    ToolRuntime {
        installed: candidate.is_some(),
        binary: candidate,
        version,
    }
}

fn detect_version(binary: &str, args: &[String]) -> Option<String> {
    if args.is_empty() {
        return None;
    }
    let output = StdCommand::new(binary).args(args).output().ok()?;
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };
    text.lines().next().map(|line| line.trim().to_string())
}

fn wizard_steps(profile: &ToolProfile, installed: bool) -> Vec<WizardStep> {
    if installed {
        return vec![
            WizardStep {
                order: 1,
                title: "Detected".to_string(),
                detail: "Torot found this tool on your system and can use it immediately.".to_string(),
            },
            WizardStep {
                order: 2,
                title: "Tune Arguments".to_string(),
                detail: "Adjust the command arguments if you want stricter scans, different templates, or custom output.".to_string(),
            },
        ];
    }

    vec![
        WizardStep {
            order: 1,
            title: "Install Or Locate".to_string(),
            detail: if profile.install_hint.trim().is_empty() {
                "Install the tool or paste the absolute binary path.".to_string()
            } else {
                format!("Install the tool first. Hint: {}", profile.install_hint)
            },
        },
        WizardStep {
            order: 2,
            title: "Set Binary Path".to_string(),
            detail: "If auto-detection misses it, add the exact executable path in Torot tool settings.".to_string(),
        },
        WizardStep {
            order: 3,
            title: "Choose Arguments".to_string(),
            detail: "Keep placeholders like {{target}}, {{target_host}}, or {{target_url}} so Torot can adapt runs automatically.".to_string(),
        },
    ]
}

fn infer_target_kind(target: &str) -> &'static str {
    let path = Path::new(target);
    if target.starts_with("http://") || target.starts_with("https://") {
        "url"
    } else if path.is_dir() {
        "directory"
    } else if path.is_file() {
        "file"
    } else {
        "host"
    }
}

fn host_from_target(target: &str) -> String {
    let raw = target
        .strip_prefix("http://")
        .or_else(|| target.strip_prefix("https://"))
        .unwrap_or(target);
    raw.split('/').next().unwrap_or(raw).to_string()
}

fn url_from_target(target: &str) -> String {
    if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else {
        format!("https://{}", target.trim_end_matches('/'))
    }
}

fn render_args(profile: &ToolProfile, target: &str, report_file: &Path) -> Option<Vec<String>> {
    let target_kind = infer_target_kind(target);
    if !profile.input_kinds.is_empty() && !profile.input_kinds.iter().any(|k| k == target_kind) {
        return None;
    }

    let host = host_from_target(target);
    let url = url_from_target(target);
    let workspace = if Path::new(target).is_dir() {
        target.to_string()
    } else {
        ".".to_string()
    };
    let report_path = report_file.to_string_lossy().to_string();

    let args = profile
        .args
        .iter()
        .map(|arg| {
            arg.replace("{{target}}", target)
                .replace("{{target_host}}", &host)
                .replace("{{target_url}}", &url)
                .replace("{{workspace}}", &workspace)
                .replace("{{report_file}}", &report_path)
        })
        .collect::<Vec<_>>();
    Some(args)
}

fn severity_rank(value: &str) -> u8 {
    match value {
        "CRITICAL" => 0,
        "HIGH" => 1,
        "MEDIUM" => 2,
        "LOW" => 3,
        _ => 4,
    }
}

fn severity_from_text(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if lower.contains("critical") {
        "CRITICAL"
    } else if lower.contains(" high ") || lower.contains("error") || lower.contains("rce") {
        "HIGH"
    } else if lower.contains("medium") || lower.contains("warning") {
        "MEDIUM"
    } else if lower.contains("low") {
        "LOW"
    } else {
        "INFO"
    }
}

fn parse_output(session_id: &str, profile: &ToolProfile, output: &str) -> Vec<Finding> {
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
    findings
        .dedup_by(|a, b| a.tool == b.tool && a.title == b.title && a.description == b.description);
    findings
}

fn parse_json(session_id: &str, profile: &ToolProfile, value: &serde_json::Value) -> Vec<Finding> {
    let tool = profile.name.as_str();
    let mut findings = Vec::new();
    match tool {
        "nuclei" => {
            if let Some(info) = value.get("info") {
                let severity = info
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "INFO".to_string());
                let mut finding = Finding::new(
                    session_id,
                    tool,
                    &format!(
                        "[nuclei] {}",
                        info.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("template hit")
                    ),
                    &severity,
                );
                finding.domain = profile.domain.clone();
                finding.description = info
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                finding.file = value
                    .get("matched-at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                findings.push(finding);
            }
        }
        "semgrep" => {
            if let Some(results) = value.get("results").and_then(|v| v.as_array()) {
                for result in results {
                    let severity = result
                        .pointer("/extra/severity")
                        .and_then(|v| v.as_str())
                        .map(|s| match s {
                            "ERROR" => "HIGH".to_string(),
                            "WARNING" => "MEDIUM".to_string(),
                            _ => "LOW".to_string(),
                        })
                        .unwrap_or_else(|| "INFO".to_string());
                    let check = result
                        .get("check_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("rule");
                    let mut finding =
                        Finding::new(session_id, tool, &format!("[semgrep] {}", check), &severity);
                    finding.domain = profile.domain.clone();
                    finding.description = result
                        .pointer("/extra/message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    finding.file = result
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    finding.line = result
                        .pointer("/start/line")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    findings.push(finding);
                }
            }
        }
        "gitleaks" => {
            if let Some(items) = value.as_array() {
                for item in items {
                    let mut finding =
                        Finding::new(session_id, tool, "[gitleaks] secret exposure", "HIGH");
                    finding.domain = profile.domain.clone();
                    finding.description = item
                        .get("Description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Potential secret exposure")
                        .to_string();
                    finding.file = item
                        .get("File")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    finding.line =
                        item.get("StartLine").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    finding.fix_suggestion =
                        "Rotate the credential, remove it from source control, and move it to a secret manager."
                            .to_string();
                    findings.push(finding);
                }
            }
        }
        "trufflehog" => {
            let verified = value
                .get("Verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if value.get("SourceMetadata").is_some() || value.get("DetectorName").is_some() {
                let mut finding = Finding::new(
                    session_id,
                    tool,
                    "[trufflehog] possible secret exposure",
                    if verified { "CRITICAL" } else { "HIGH" },
                );
                finding.domain = profile.domain.clone();
                finding.description = format!(
                    "Detector: {}",
                    value
                        .get("DetectorName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                );
                findings.push(finding);
            }
        }
        "httpx" | "subfinder" | "amass" | "katana" => {
            if let Some(url) = value
                .get("url")
                .and_then(|v| v.as_str())
                .or_else(|| value.get("host").and_then(|v| v.as_str()))
                .or_else(|| value.get("name").and_then(|v| v.as_str()))
            {
                let mut finding =
                    Finding::new(session_id, tool, &format!("[{}] discovery", tool), "INFO");
                finding.domain = profile.domain.clone();
                finding.description = url.to_string();
                findings.push(finding);
            }
        }
        _ => {}
    }
    findings
}

fn parse_text(session_id: &str, profile: &ToolProfile, output: &str) -> Vec<Finding> {
    let keywords = [
        "critical",
        "high",
        "medium",
        "warning",
        "error",
        "exposed",
        "vulnerable",
        "sql injection",
        "xss",
        "ssrf",
        "takeover",
        "open port",
        "directory listing",
        "default credentials",
        "secret",
        "token",
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
                finding.fix_suggestion = "Validate the injection manually and move the affected parameter to prepared statements.".to_string();
                finding.impact = "Potential database read/write compromise.".to_string();
            }
            Some(finding)
        })
        .collect()
}

fn emit_line(
    app: &AppHandle,
    session_id: &str,
    kind: &str,
    tool: &str,
    line: &str,
    severity: Option<String>,
) {
    let _ = app.emit(
        "stream_line",
        StreamLine {
            session_id: session_id.to_string(),
            tool: tool.to_string(),
            line: line.to_string(),
            kind: kind.to_string(),
            severity,
        },
    );
}

fn summarize_findings(findings: &[Finding]) -> String {
    let critical = findings.iter().filter(|f| f.severity == "CRITICAL").count();
    let high = findings.iter().filter(|f| f.severity == "HIGH").count();
    let medium = findings.iter().filter(|f| f.severity == "MEDIUM").count();
    let unique_tools = findings
        .iter()
        .map(|f| f.tool.clone())
        .collect::<HashSet<_>>()
        .len();

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

fn render_findings_table(findings: &[Finding]) -> String {
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
            evidence.replace('|', "/")
        ));
    }
    lines.join("\n")
}

fn render_tool_overview(findings: &[Finding]) -> String {
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

fn render_report(template: &str, session: &Session, findings: &[Finding]) -> String {
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

fn report_placeholders() -> Vec<String> {
    vec![
        "{{session_id}}".to_string(),
        "{{target}}".to_string(),
        "{{created_at}}".to_string(),
        "{{findings_total}}".to_string(),
        "{{critical_count}}".to_string(),
        "{{high_count}}".to_string(),
        "{{summary}}".to_string(),
        "{{tool_overview}}".to_string(),
        "{{findings_table}}".to_string(),
    ]
}

#[tauri::command]
async fn get_app_info(state: State<'_, Arc<AppState>>) -> Result<AppInfo, String> {
    let config = load_config(&state).map_err(|e| e.to_string())?;
    Ok(AppInfo {
        version: TOROT_VERSION.to_string(),
        install_mode: config.install_mode,
        cli_supported: true,
        knowledge_topics: config.knowledge_topics,
        report_template_placeholders: report_placeholders(),
    })
}

#[tauri::command]
async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    load_config(&state).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_settings(
    config: AppConfig,
    state: State<'_, Arc<AppState>>,
) -> Result<AppConfig, String> {
    let mut merged = config;
    merged.version = TOROT_VERSION.to_string();
    merged.tools = merge_builtin_tools(merged.tools);
    save_config(&state, &merged).map_err(|e| e.to_string())?;
    Ok(merged)
}

#[tauri::command]
async fn get_tools(state: State<'_, Arc<AppState>>) -> Result<Vec<ToolStatus>, String> {
    let config = load_config(&state).map_err(|e| e.to_string())?;
    Ok(tool_statuses(&config))
}

#[tauri::command]
async fn save_tool_profile(
    profile: ToolProfileInput,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ToolStatus>, String> {
    let mut config = load_config(&state).map_err(|e| e.to_string())?;
    let tool = ToolProfile {
        name: profile.name.trim().to_string(),
        domain: profile.domain.trim().to_string(),
        description: profile.description.trim().to_string(),
        binary_names: profile.binary_names,
        path_override: profile.path_override.filter(|v| !v.trim().is_empty()),
        args: profile.args,
        version_args: profile.version_args,
        install_hint: profile.install_hint,
        output_format: profile.output_format,
        input_kinds: profile.input_kinds,
        source: "custom".to_string(),
        auto_detect: true,
        enabled: profile.enabled,
        timeout_seconds: profile.timeout_seconds.max(30),
        capabilities: profile.capabilities,
        knowledge: profile.knowledge,
    };

    if let Some(existing) = config.tools.iter_mut().find(|t| t.name == tool.name) {
        *existing = tool;
    } else {
        config.tools.push(tool);
    }
    config.tools = merge_builtin_tools(config.tools);
    save_config(&state, &config).map_err(|e| e.to_string())?;
    Ok(tool_statuses(&config))
}

#[tauri::command]
async fn generate_report(
    request: ReportRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<ReportResult, String> {
    let config = load_config(&state).map_err(|e| e.to_string())?;
    let session = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&request.session_id).cloned()
    }
    .or_else(|| {
        load_session_from_db(&state, &request.session_id)
            .ok()
            .flatten()
    })
    .ok_or_else(|| "Session not found.".to_string())?;
    let findings = get_findings_internal(&request.session_id, &state);
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
    fs::write(&path, markdown).map_err(|e| e.to_string())?;
    Ok(ReportResult {
        session_id: session.id,
        path,
        summary: summarize_findings(&findings),
    })
}

#[tauri::command]
async fn start_scan(
    request: ScanRequest,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = load_config(&state).map_err(|e| e.to_string())?;
    let mut session = Session::new(&request.target, &request.mode);
    let session_id = session.id.clone();
    session.report_path = request.report_output_path.clone();

    {
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session.clone());
        *state.active_scan.lock().unwrap() = Some(session_id.clone());
    }
    {
        let db = state.db.lock().unwrap();
        let _ = db.execute(
            "INSERT OR REPLACE INTO sessions (id,target,domain,start_time,end_time,total_findings,summary) VALUES (?1,?2,?3,?4,0,0,'')",
            params![&session_id, &request.target, infer_target_kind(&request.target), session.start_time],
        );
    }

    emit_line(
        &app,
        &session_id,
        "system",
        "torot",
        &format!(
            "torot v{} starting {} scan against {}",
            TOROT_VERSION, request.mode, request.target
        ),
        None,
    );

    let selected_tools = if request.tools.is_empty() {
        suggest_tools(&config, &request.target)
    } else {
        request.tools.clone()
    };

    let state_handle = Arc::clone(state.inner());
    tokio::spawn(async move {
        run_pipeline(
            session_id,
            request,
            selected_tools,
            config,
            app,
            state_handle,
        )
        .await;
    });

    Ok(session.id)
}

fn suggest_tools(config: &AppConfig, target: &str) -> Vec<String> {
    let kind = infer_target_kind(target);
    config
        .tools
        .iter()
        .filter(|tool| tool.enabled)
        .filter(|tool| {
            if tool.input_kinds.is_empty() {
                return true;
            }
            tool.input_kinds.iter().any(|k| k == kind)
        })
        .filter(|tool| detect_tool(tool).installed)
        .take(4)
        .map(|tool| tool.name.clone())
        .collect()
}

async fn run_pipeline(
    session_id: String,
    request: ScanRequest,
    selected_tools: Vec<String>,
    config: AppConfig,
    app: AppHandle,
    state: Arc<AppState>,
) {
    if selected_tools.is_empty() {
        emit_line(
            &app,
            &session_id,
            "system",
            "torot",
            "No compatible installed tools were found for this target.",
            Some("HIGH".to_string()),
        );
        let _ = app.emit(
            "scan_complete",
            serde_json::json!({ "session_id": session_id, "total": 0, "report_path": null }),
        );
        return;
    }

    emit_line(
        &app,
        &session_id,
        "system",
        "torot",
        &format!(
            "Launching {} tool(s) in parallel: {}",
            selected_tools.len(),
            selected_tools.join(", ")
        ),
        None,
    );

    let tools_by_name: HashMap<String, ToolProfile> = config
        .tools
        .iter()
        .cloned()
        .map(|tool| (tool.name.clone(), tool))
        .collect();

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    for name in selected_tools {
        let Some(profile) = tools_by_name.get(&name).cloned() else {
            emit_line(
                &app,
                &session_id,
                "system",
                "torot",
                &format!("Unknown tool `{}` skipped.", name),
                Some("MEDIUM".to_string()),
            );
            continue;
        };
        let app_clone = app.clone();
        let state_clone = Arc::clone(&state);
        let session_clone = session_id.clone();
        let target_clone = request.target.clone();
        let max_runtime = config
            .sandbox
            .max_runtime_seconds
            .min(profile.timeout_seconds)
            .max(30);
        tasks.push(tokio::spawn(async move {
            run_tool(
                &session_clone,
                &target_clone,
                &profile,
                max_runtime,
                app_clone,
                state_clone,
            )
            .await;
        }));
    }

    for task in tasks {
        let _ = task.await;
    }

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
                    state
                        .reports_dir
                        .join(format!("{}.md", session.id))
                        .to_string_lossy()
                        .to_string()
                });
                let markdown = render_report(&report_template, &session, &findings);
                if fs::write(&path, markdown).is_ok() {
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
    {
        let db = state.db.lock().unwrap();
        let _ = db.execute(
            "UPDATE sessions SET end_time=?1, total_findings=?2, summary=?3 WHERE id=?4",
            params![end_time, findings.len() as u32, summary, &session_id],
        );
    }

    emit_line(
        &app,
        &session_id,
        "system",
        "torot",
        &format!(
            "Scan complete. {}{}",
            summarize_findings(&findings),
            report_path
                .as_ref()
                .map(|p| format!(" Report: {}", p))
                .unwrap_or_default()
        ),
        None,
    );
    let _ = app.emit(
        "scan_complete",
        serde_json::json!({ "session_id": session_id, "total": findings.len(), "report_path": report_path }),
    );
}

fn get_findings_internal(session_id: &str, state: &AppState) -> Vec<Finding> {
    let db = state.db.lock().unwrap();
    let mut stmt = match db.prepare(
        "SELECT id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp
        FROM findings WHERE session_id=?1
        ORDER BY CASE severity WHEN 'CRITICAL' THEN 0 WHEN 'HIGH' THEN 1 WHEN 'MEDIUM' THEN 2 WHEN 'LOW' THEN 3 ELSE 4 END",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map([session_id], |r| {
        Ok(Finding {
            id: r.get(0)?,
            session_id: r.get(1)?,
            tool: r.get(2)?,
            title: r.get(3)?,
            severity: r.get(4)?,
            domain: r.get(5)?,
            description: r.get(6)?,
            file: r.get(7)?,
            line: r.get(8)?,
            code_snippet: r.get(9)?,
            fix_suggestion: r.get(10)?,
            impact: r.get(11)?,
            bug_type: r.get(12)?,
            timestamp: r.get(13)?,
        })
    }) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };
    rows.filter_map(|row| row.ok()).collect()
}

fn load_session_from_db(state: &AppState, session_id: &str) -> Result<Option<Session>> {
    let db = state.db.lock().unwrap();
    let mut stmt =
        db.prepare("SELECT id,target,domain,start_time FROM sessions WHERE id=?1 LIMIT 1")?;
    let mut rows = stmt.query([session_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Session {
            id: row.get(0)?,
            target: row.get(1)?,
            mode: row.get::<_, String>(2)?,
            start_time: row.get(3)?,
            findings: Vec::new(),
            report_path: None,
        }))
    } else {
        Ok(None)
    }
}

async fn run_tool(
    session_id: &str,
    target: &str,
    profile: &ToolProfile,
    max_runtime_seconds: u64,
    app: AppHandle,
    state: Arc<AppState>,
) {
    let runtime = detect_tool(profile);
    let Some(binary) = runtime.binary else {
        emit_line(
            &app,
            session_id,
            "system",
            &profile.name,
            &format!("{} not installed. {}", profile.name, profile.install_hint),
            Some("MEDIUM".to_string()),
        );
        return;
    };

    let report_file = state
        .reports_dir
        .join(format!("{}-{}.out", session_id, profile.name));
    let Some(args) = render_args(profile, target, &report_file) else {
        emit_line(
            &app,
            session_id,
            "system",
            &profile.name,
            "Target type does not match this tool's supported inputs.",
            Some("LOW".to_string()),
        );
        return;
    };

    emit_line(
        &app,
        session_id,
        "system",
        &profile.name,
        &format!("Starting {} with {}", profile.name, args.join(" ")),
        None,
    );

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
    command.current_dir(if Path::new(target).is_dir() {
        target
    } else {
        "."
    });

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            emit_line(
                &app,
                session_id,
                "system",
                &profile.name,
                &format!("Launch failed: {}", err),
                Some("HIGH".to_string()),
            );
            return;
        }
    };

    let mut reader_handles: Vec<JoinHandle<Vec<String>>> = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let app_clone = app.clone();
        let sid = session_id.to_string();
        let tool = profile.name.clone();
        reader_handles.push(tokio::spawn(async move {
            stream_reader(stdout, app_clone, sid, tool).await
        }));
    }
    if let Some(stderr) = child.stderr.take() {
        let app_clone = app.clone();
        let sid = session_id.to_string();
        let tool = profile.name.clone();
        reader_handles.push(tokio::spawn(async move {
            stream_reader(stderr, app_clone, sid, tool).await
        }));
    }

    let wait_result =
        tokio::time::timeout(Duration::from_secs(max_runtime_seconds), child.wait()).await;
    if wait_result.is_err() {
        let _ = child.kill().await;
        emit_line(
            &app,
            session_id,
            "system",
            &profile.name,
            &format!("Timed out after {} seconds.", max_runtime_seconds),
            Some("HIGH".to_string()),
        );
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
        let _ = state.db.lock().unwrap().execute(
            "INSERT OR IGNORE INTO findings (id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                &finding.id,
                &finding.session_id,
                &finding.tool,
                &finding.title,
                &finding.severity,
                &finding.domain,
                &finding.description,
                &finding.file,
                finding.line,
                &finding.code_snippet,
                &finding.fix_suggestion,
                &finding.impact,
                &finding.bug_type,
                finding.timestamp
            ],
        );
        if let Some(session) = state.sessions.lock().unwrap().get_mut(session_id) {
            session.findings.push(finding.clone());
        }
        let _ = app.emit("new_finding", finding);
    }

    emit_line(
        &app,
        session_id,
        "system",
        &profile.name,
        &format!(
            "{} complete with {} parsed finding(s).",
            profile.name,
            findings.len()
        ),
        None,
    );
}

async fn stream_reader<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    app: AppHandle,
    session_id: String,
    tool: String,
) -> Vec<String> {
    let mut lines = BufReader::new(reader).lines();
    let mut output = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        emit_line(&app, &session_id, "output", &tool, &line, None);
        output.push(line);
    }
    output
}

#[tauri::command]
async fn get_sessions(state: State<'_, Arc<AppState>>) -> Result<Vec<DbSession>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id,target,domain,start_time,end_time,total_findings,summary FROM sessions ORDER BY start_time DESC LIMIT 100")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(DbSession {
                id: r.get(0)?,
                target: r.get(1)?,
                domain: r.get(2)?,
                start_time: r.get(3)?,
                end_time: r.get(4)?,
                total_findings: r.get(5)?,
                summary: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|row| row.ok()).collect())
}

#[tauri::command]
async fn get_findings(
    session_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Finding>, String> {
    Ok(get_findings_internal(&session_id, &state))
}

#[tauri::command]
async fn stop_scan(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.active_scan.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
async fn get_db_stats(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().unwrap();
    let sessions: i64 = db
        .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
        .unwrap_or(0);
    let findings: i64 = db
        .query_row("SELECT COUNT(*) FROM findings", [], |r| r.get(0))
        .unwrap_or(0);
    let critical: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE severity='CRITICAL'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let high: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE severity='HIGH'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(serde_json::json!({
        "sessions": sessions,
        "findings": findings,
        "critical": critical,
        "high": high
    }))
}

fn cli_usage() -> &'static str {
    "torot 4.0.0

Usage:
  torot tools
  torot scan --target <value> [--tools nmap,bbot] [--template-file report.md] [--mode single]
  torot report --session <id> [--template-file report.md] [--output report.md]"
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

async fn run_tool_cli(
    session_id: &str,
    target: &str,
    profile: ToolProfile,
    reports_dir: PathBuf,
) -> Vec<Finding> {
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
            eprintln!("{} failed: {}", profile.name, err);
            Vec::new()
        }
        Err(_) => {
            eprintln!("{} timed out", profile.name);
            Vec::new()
        }
    }
}

async fn run_pipeline_cli(
    state: Arc<AppState>,
    request: ScanRequest,
    config: AppConfig,
) -> Result<String> {
    let session = Session::new(&request.target, &request.mode);
    let session_id = session.id.clone();
    state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id.clone(), session.clone());
    {
        let db = state.db.lock().unwrap();
        let _ = db.execute(
            "INSERT OR REPLACE INTO sessions (id,target,domain,start_time,end_time,total_findings,summary) VALUES (?1,?2,?3,?4,0,0,'')",
            params![&session_id, &request.target, infer_target_kind(&request.target), session.start_time],
        );
    }

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
            let _ = db.execute(
                "INSERT OR IGNORE INTO findings (id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp)
                VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    &finding.id,
                    &finding.session_id,
                    &finding.tool,
                    &finding.title,
                    &finding.severity,
                    &finding.domain,
                    &finding.description,
                    &finding.file,
                    finding.line,
                    &finding.code_snippet,
                    &finding.fix_suggestion,
                    &finding.impact,
                    &finding.bug_type,
                    finding.timestamp
                ],
            );
        }
        let _ = db.execute(
            "UPDATE sessions SET end_time=?1, total_findings=?2, summary=?3 WHERE id=?4",
            params![
                now_unix(),
                findings.len() as u32,
                summarize_findings(&findings),
                &session_id
            ],
        );
    }

    let template = request
        .report_template
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(config.default_report_template.clone());
    let report_path = request.report_output_path.clone().unwrap_or_else(|| {
        state
            .reports_dir
            .join(format!("{}.md", session_id))
            .to_string_lossy()
            .to_string()
    });
    let markdown = render_report(&template, &session, &findings);
    fs::write(&report_path, markdown)?;
    println!("[torot] report written to {}", report_path);
    println!("[torot] {}", summarize_findings(&findings));
    Ok(session_id)
}

pub fn try_run_cli() -> Result<bool> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().cloned() else {
        return Ok(false);
    };
    if !matches!(command.as_str(), "tools" | "scan" | "report" | "config") {
        return Ok(false);
    }

    let state = Arc::new(AppState::new()?);
    let runtime =
        tokio::runtime::Runtime::new().context("Failed to start tokio runtime for CLI.")?;
    runtime.block_on(async move {
        match command.as_str() {
            "tools" => {
                let config = load_config(&state)?;
                let statuses = tool_statuses(&config);
                for tool in statuses {
                    println!(
                        "{:12} {:9} {:20} {}",
                        tool.name,
                        if tool.installed {
                            "installed"
                        } else {
                            "missing"
                        },
                        tool.version,
                        tool.binary
                    );
                }
            }
            "scan" => {
                let target = cli_arg(&args, "--target")
                    .ok_or_else(|| anyhow::anyhow!("Missing --target"))?;
                let tools = cli_tools(&args);
                let template = cli_arg(&args, "--template-file")
                    .and_then(|path| fs::read_to_string(path).ok());
                let mode = cli_arg(&args, "--mode").unwrap_or_else(|| "single".to_string());
                let request = ScanRequest {
                    target,
                    mode,
                    tools,
                    report_template: template,
                    report_output_path: cli_arg(&args, "--output"),
                };
                let config = load_config(&state)?;
                let _ = run_pipeline_cli(Arc::clone(&state), request, config).await?;
            }
            "report" => {
                let session_id = cli_arg(&args, "--session")
                    .ok_or_else(|| anyhow::anyhow!("Missing --session"))?;
                let findings = get_findings_internal(&session_id, &state);
                let session = load_session_from_db(&state, &session_id)?
                    .ok_or_else(|| anyhow::anyhow!("Session not found."))?;
                let config = load_config(&state)?;
                let template = cli_arg(&args, "--template-file")
                    .and_then(|path| fs::read_to_string(path).ok())
                    .unwrap_or(config.default_report_template);
                let body = render_report(&template, &session, &findings);
                let output = cli_arg(&args, "--output").unwrap_or_else(|| {
                    state
                        .reports_dir
                        .join(format!("{}.md", session_id))
                        .to_string_lossy()
                        .to_string()
                });
                fs::write(&output, body)?;
                println!("report written to {}", output);
            }
            "config" => {
                let config = load_config(&state)?;
                println!("{}", serde_json::to_string_pretty(&config)?);
            }
            _ => {
                println!("{}", cli_usage());
            }
        }
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState::new().expect("AppState init failed"));
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_settings,
            save_settings,
            get_tools,
            save_tool_profile,
            start_scan,
            stop_scan,
            get_sessions,
            get_findings,
            get_db_stats,
            generate_report
        ])
        .run(tauri::generate_context!())
        .expect("error running Torot");
}
