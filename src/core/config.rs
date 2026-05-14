use crate::core::types::{self, ToolProfile, AppConfig, SandboxConfig};
use crate::core::state::AppState;
use crate::core::knowledge::builtin_knowledge_topics;
use anyhow::Result;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: types::TOROT_VERSION.to_string(),
            install_mode: "both".to_string(),
            default_report_template: default_report_template(),
            sandbox: SandboxConfig::default(),
            tools: builtin_tools(),
            knowledge_topics: builtin_knowledge_topics(),
        }
    }
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

pub fn default_report_template() -> String {
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

pub fn report_placeholders() -> Vec<String> {
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

pub fn builtin_tools() -> Vec<ToolProfile> {
    vec![
        t("nmap", "webapp", "Host and service discovery.", &["nmap"], &["-sV", "-Pn", "{{target_host}}"], &["--version"], "Install nmap with your package manager.", "text", &["host", "url"], 900, &["recon", "port-scan", "service-detection"], &["attack-surface-mapping", "network-recon"]),
        t("bbot", "webapp", "Asset discovery and recon automation.", &["bbot"], &["-t", "{{target_host}}", "-f", "subdomain-enum", "web-basic", "-y"], &["--version"], "pipx install bbot", "text", &["host", "url"], 1200, &["recon", "subdomains", "web-enum"], &["subdomain-enumeration", "attack-surface-mapping"]),
        t("nuclei", "webapp", "Template-based vulnerability scanning.", &["nuclei"], &["-target", "{{target_url}}", "-jsonl"], &["-version"], "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest", "jsonl", &["url", "host"], 900, &["vuln-scan", "templates"], &["web-application-testing"]),
        t("httpx", "webapp", "HTTP probing and metadata discovery.", &["httpx"], &["-u", "{{target_url}}", "-json"], &["-version"], "go install github.com/projectdiscovery/httpx/cmd/httpx@latest", "jsonl", &["url", "host"], 300, &["recon", "http-probing"], &["attack-surface-mapping"]),
        t("subfinder", "webapp", "Passive subdomain enumeration.", &["subfinder"], &["-d", "{{target_host}}", "-silent", "-oJ"], &["-version"], "go install github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest", "jsonl", &["host", "url"], 600, &["recon", "subdomains"], &["subdomain-enumeration"]),
        t("amass", "webapp", "DNS intelligence and enumeration.", &["amass"], &["enum", "-passive", "-d", "{{target_host}}", "-json", "-"], &["-version"], "go install github.com/owasp-amass/amass/v4/...@master", "json", &["host", "url"], 1200, &["recon", "subdomains", "dns"], &["subdomain-enumeration"]),
        t("katana", "webapp", "Web crawling and endpoint enumeration.", &["katana"], &["-u", "{{target_url}}", "-jsonl"], &["-version"], "go install github.com/projectdiscovery/katana/cmd/katana@latest", "jsonl", &["url"], 900, &["crawl", "content-discovery"], &["attack-surface-mapping"]),
        t("ffuf", "webapp", "Directory and parameter fuzzing.", &["ffuf"], &["-u", "{{target_url}}/FUZZ", "-w", "/usr/share/wordlists/dirb/common.txt", "-mc", "all"], &["-V"], "go install github.com/ffuf/ffuf/v2@latest", "text", &["url"], 900, &["fuzz", "content-discovery"], &["web-application-testing"]),
        t("gobuster", "webapp", "Directory brute forcing.", &["gobuster"], &["dir", "-u", "{{target_url}}", "-w", "/usr/share/wordlists/dirb/common.txt"], &["version"], "go install github.com/OJ/gobuster/v3@latest", "text", &["url"], 900, &["fuzz", "content-discovery"], &["web-application-testing"]),
        t("nikto", "webapp", "Baseline web server checks.", &["nikto"], &["-h", "{{target_url}}", "-Format", "txt"], &["-Version"], "Install nikto with your package manager.", "text", &["url"], 900, &["vuln-scan", "web-baseline"], &["web-application-testing"]),
        t("sqlmap", "api", "Automated SQL injection verification.", &["sqlmap"], &["-u", "{{target_url}}", "--batch", "--level", "2"], &["--version"], "pipx install sqlmap", "text", &["url"], 1200, &["sqli", "verification"], &["api-security", "web-application-testing"]),
        t("semgrep", "general", "Static analysis for code and config.", &["semgrep"], &["--config", "auto", "--json", "{{target}}"], &["--version"], "pipx install semgrep", "json", &["directory", "file"], 900, &["static-analysis", "code-review"], &["web-application-testing", "api-security"]),
        t("trufflehog", "general", "Secrets discovery in repos and filesystems.", &["trufflehog"], &["filesystem", "{{target}}", "--json"], &["--version"], "go install github.com/trufflesecurity/trufflehog/v3@latest", "jsonl", &["directory", "file"], 900, &["secrets", "leaks"], &["secrets-exposure"]),
        t("gitleaks", "general", "High-signal secret scanning.", &["gitleaks"], &["detect", "--source", "{{target}}", "--report-format", "json"], &["version"], "go install github.com/zricethezav/gitleaks/v8@latest", "json", &["directory", "file"], 900, &["secrets", "leaks"], &["secrets-exposure"]),
    ]
}

fn t(name: &str, domain: &str, description: &str, binaries: &[&str], args: &[&str], version_args: &[&str], install_hint: &str, output_format: &str, input_kinds: &[&str], timeout: u64, capabilities: &[&str], knowledge: &[&str]) -> ToolProfile {
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
        timeout_seconds: timeout,
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        knowledge: knowledge.iter().map(|s| s.to_string()).collect(),
    }
}

fn merge_builtin_tools(existing: Vec<ToolProfile>) -> Vec<ToolProfile> {
    let mut merged = builtin_tools();
    let mut index: HashMap<String, usize> = merged.iter().enumerate().map(|(i, t)| (t.name.clone(), i)).collect();
    for tool in existing {
        if let Some(pos) = index.get(&tool.name).copied() {
            merged[pos].path_override = tool.path_override;
            merged[pos].args = if tool.args.is_empty() { merged[pos].args.clone() } else { tool.args };
            merged[pos].version_args = if tool.version_args.is_empty() { merged[pos].version_args.clone() } else { tool.version_args };
            merged[pos].install_hint = if tool.install_hint.trim().is_empty() { merged[pos].install_hint.clone() } else { tool.install_hint };
            merged[pos].output_format = if tool.output_format.trim().is_empty() { merged[pos].output_format.clone() } else { tool.output_format };
            merged[pos].input_kinds = if tool.input_kinds.is_empty() { merged[pos].input_kinds.clone() } else { tool.input_kinds };
            merged[pos].enabled = tool.enabled;
            merged[pos].timeout_seconds = tool.timeout_seconds.max(30);
            merged[pos].capabilities = if tool.capabilities.is_empty() { merged[pos].capabilities.clone() } else { tool.capabilities };
            merged[pos].knowledge = if tool.knowledge.is_empty() { merged[pos].knowledge.clone() } else { tool.knowledge };
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

pub fn ensure_config_file(path: &Path) -> Result<()> {
    if path.exists() { return Ok(()); }
    let config = AppConfig::default();
    save_config_to_path(path, &config)
}

fn load_config_from_path(path: &Path) -> Result<AppConfig> {
    let raw = fs::read_to_string(path)?;
    let mut parsed: AppConfig = serde_json::from_str(&raw).unwrap_or_default();
    parsed.version = types::TOROT_VERSION.to_string();
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

pub fn load_config(state: &AppState) -> Result<AppConfig> {
    load_config_from_path(&state.config_path)
}

pub fn save_config(state: &AppState, config: &AppConfig) -> Result<()> {
    save_config_to_path(&state.config_path, config)
}

pub fn save_settings(config: AppConfig, state: &AppState) -> Result<AppConfig, String> {
    let mut merged = config;
    merged.version = types::TOROT_VERSION.to_string();
    merged.tools = merge_builtin_tools(merged.tools);
    save_config(state, &merged).map_err(|e| e.to_string())?;
    Ok(merged)
}

pub fn get_settings(state: &AppState) -> Result<AppConfig, String> {
    load_config(state).map_err(|e| e.to_string())
}
