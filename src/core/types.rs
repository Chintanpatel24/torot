use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TOROT_VERSION: &str = "4.0.0";

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
            timestamp: crate::util::time::now_unix(),
        }
    }

    pub fn severity_rank(&self) -> u8 {
        match self.severity.as_str() {
            "CRITICAL" => 0,
            "HIGH" => 1,
            "MEDIUM" => 2,
            "LOW" => 3,
            _ => 4,
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
            start_time: crate::util::time::now_unix(),
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
pub struct AppConfig {
    pub version: String,
    pub install_mode: String,
    pub default_report_template: String,
    pub sandbox: SandboxConfig,
    pub tools: Vec<ToolProfile>,
    pub knowledge_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub profile: String,
    pub max_runtime_seconds: u64,
    pub allow_network: bool,
    pub writable_reports_only: bool,
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
