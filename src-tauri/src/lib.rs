use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;
use rusqlite::{Connection, params};


// ─────────────────────────────────────────────────────────────────────────────
// Data Models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id:             String,
    pub session_id:     String,
    pub tool:           String,
    pub title:          String,
    pub severity:       String,
    pub domain:         String,
    pub description:    String,
    pub file:           String,
    pub line:           u32,
    pub code_snippet:   String,
    pub fix_suggestion: String,
    pub impact:         String,
    pub bug_type:       String,
    pub timestamp:      u64,
}

impl Finding {
    pub fn new(session_id: &str, tool: &str, title: &str, severity: &str) -> Self {
        Self {
            id:             Uuid::new_v4().to_string(),
            session_id:     session_id.to_string(),
            tool:           tool.to_string(),
            title:          title.to_string(),
            severity:       severity.to_string(),
            domain:         String::new(),
            description:    String::new(),
            file:           String::new(),
            line:           0,
            code_snippet:   String::new(),
            fix_suggestion: String::new(),
            impact:         String::new(),
            bug_type:       String::new(),
            timestamp:      now_unix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id:         String,
    pub target:     String,
    pub mode:       String,   // "single" | "loop" | "daemon"
    pub domain:     String,
    pub start_time: u64,
    pub end_time:   u64,
    pub status:     String,
    pub findings:   Vec<Finding>,
}

impl Session {
    pub fn new(target: &str, mode: &str) -> Self {
        Self {
            id:         Uuid::new_v4().to_string()[..12].to_string(),
            target:     target.to_string(),
            mode:       mode.to_string(),
            domain:     String::new(),
            start_time: now_unix(),
            end_time:   0,
            status:     "running".to_string(),
            findings:   Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub name:      String,
    pub installed: bool,
    pub binary:    String,
    pub domain:    String,
    pub version:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id:          String,
    pub tool:        String,
    pub status:      String,   // "pending"|"running"|"done"|"failed"|"skipped"
    pub output_lines: Vec<String>,
    pub findings:    Vec<Finding>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamLine {
    pub session_id: String,
    pub tool:       String,
    pub line:       String,
    pub kind:       String,  // "output"|"finding"|"system"|"agent"
    pub severity:   Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPlan {
    pub session_id: String,
    pub goal:       String,
    pub steps:      Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id:          String,
    pub title:       String,
    pub description: String,
    pub tool:        String,
    pub status:      String,
    pub approved:    bool,
}

impl PlanStep {
    pub fn new(title: &str, description: &str, tool: &str) -> Self {
        Self {
            id:          Uuid::new_v4().to_string()[..8].to_string(),
            title:       title.to_string(),
            description: description.to_string(),
            tool:        tool.to_string(),
            status:      "pending".to_string(),
            approved:    false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSession {
    pub id:             String,
    pub target:         String,
    pub domain:         String,
    pub start_time:     u64,
    pub end_time:       u64,
    pub total_findings: u32,
    pub summary:        String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared State
// ─────────────────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db:           Mutex<Connection>,
    pub sessions:     Mutex<HashMap<String, Session>>,
    pub active_scan:  Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let db_path = dirs_path();
        std::fs::create_dir_all(&db_path).ok();
        let conn = Connection::open(db_path.join("memory.db"))?;
        init_schema(&conn)?;
        Ok(Self {
            db:          Mutex::new(conn),
            sessions:    Mutex::new(HashMap::new()),
            active_scan: Mutex::new(None),
        })
    }
}

fn dirs_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".torot")
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS sessions (
            id            TEXT PRIMARY KEY,
            target        TEXT,
            domain        TEXT,
            start_time    INTEGER,
            end_time      INTEGER,
            total_findings INTEGER DEFAULT 0,
            summary       TEXT
        );

        CREATE TABLE IF NOT EXISTS findings (
            id            TEXT PRIMARY KEY,
            session_id    TEXT,
            tool          TEXT,
            title         TEXT,
            severity      TEXT,
            domain        TEXT,
            description   TEXT,
            file          TEXT,
            line          INTEGER,
            code_snippet  TEXT,
            fix_suggestion TEXT,
            impact        TEXT,
            bug_type      TEXT,
            timestamp     INTEGER
        );

        CREATE TABLE IF NOT EXISTS knowledge (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            topic    TEXT,
            content  TEXT,
            source   TEXT,
            added_at INTEGER
        );
    ")?;
    Ok(())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool Definitions
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ToolDef {
    name:        &'static str,
    binaries:    &'static [&'static str],
    domain:      &'static str,
    description: &'static str,
    install:     &'static str,
}

const ALL_TOOLS: &[ToolDef] = &[
    // Blockchain
    ToolDef { name: "slither",    binaries: &["slither"],          domain: "blockchain", description: "Static analysis — reentrancy, overflow, access control", install: "pip install slither-analyzer" },
    ToolDef { name: "aderyn",     binaries: &["aderyn"],           domain: "blockchain", description: "Rust-based multi-contract analyzer", install: "cargo install aderyn" },
    ToolDef { name: "mythril",    binaries: &["myth"],             domain: "blockchain", description: "Symbolic execution for EVM bytecode", install: "pip install mythril" },
    ToolDef { name: "echidna",    binaries: &["echidna","echidna-test"], domain: "blockchain", description: "Property-based fuzzer for Solidity", install: "brew install echidna" },
    ToolDef { name: "manticore",  binaries: &["manticore"],        domain: "blockchain", description: "Binary analysis via symbolic execution", install: "pip install manticore" },
    ToolDef { name: "solhint",    binaries: &["solhint"],          domain: "blockchain", description: "Solidity linter", install: "npm install -g solhint" },
    ToolDef { name: "halmos",     binaries: &["halmos"],           domain: "blockchain", description: "Bounded model checker via SMT", install: "pip install halmos" },
    ToolDef { name: "semgrep",    binaries: &["semgrep"],          domain: "blockchain", description: "Pattern-based static analysis", install: "pip install semgrep" },
    ToolDef { name: "solc",       binaries: &["solc"],             domain: "blockchain", description: "Solidity compiler warnings", install: "pip install solc-select" },
    ToolDef { name: "wake",       binaries: &["wake"],             domain: "blockchain", description: "Solidity analysis framework", install: "pip install eth-wake" },
    ToolDef { name: "heimdall",   binaries: &["heimdall"],         domain: "blockchain", description: "EVM bytecode decompiler", install: "cargo install heimdall-rs" },
    ToolDef { name: "cargo-audit",binaries: &["cargo-audit"],      domain: "blockchain", description: "Rust dependency audit", install: "cargo install cargo-audit" },
    ToolDef { name: "clippy",     binaries: &["cargo"],            domain: "blockchain", description: "Rust linter via cargo clippy", install: "rustup component add clippy" },
    // Web App
    ToolDef { name: "nuclei",     binaries: &["nuclei"],           domain: "webapp", description: "Template-based vulnerability scanner", install: "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest" },
    ToolDef { name: "nikto",      binaries: &["nikto"],            domain: "webapp", description: "Web server vulnerability scanner", install: "apt install nikto" },
    ToolDef { name: "sqlmap",     binaries: &["sqlmap"],           domain: "webapp", description: "SQL injection scanner", install: "pip install sqlmap" },
    ToolDef { name: "ffuf",       binaries: &["ffuf"],             domain: "webapp", description: "Fast web fuzzer", install: "go install github.com/ffuf/ffuf/v2@latest" },
    ToolDef { name: "gobuster",   binaries: &["gobuster"],         domain: "webapp", description: "Directory brute-force", install: "go install github.com/OJ/gobuster/v3@latest" },
    ToolDef { name: "dalfox",     binaries: &["dalfox"],           domain: "webapp", description: "XSS scanner", install: "go install github.com/hahwul/dalfox/v2@latest" },
    ToolDef { name: "trufflehog", binaries: &["trufflehog"],       domain: "webapp", description: "Secret scanner", install: "go install github.com/trufflesecurity/trufflehog/v3@latest" },
    ToolDef { name: "gitleaks",   binaries: &["gitleaks"],         domain: "webapp", description: "Git secret scanner", install: "go install github.com/zricethezav/gitleaks/v8@latest" },
    // API
    ToolDef { name: "arjun",      binaries: &["arjun"],            domain: "api", description: "HTTP parameter discovery", install: "pip install arjun" },
    ToolDef { name: "jwt_tool",   binaries: &["jwt_tool","jwt-tool"], domain: "api", description: "JWT attack toolkit", install: "pip install jwt_tool" },
    // Binary
    ToolDef { name: "radare2",    binaries: &["r2"],               domain: "binary", description: "Reverse engineering framework", install: "brew install radare2" },
    ToolDef { name: "binwalk",    binaries: &["binwalk"],          domain: "binary", description: "Firmware analysis", install: "pip install binwalk" },
    ToolDef { name: "checksec",   binaries: &["checksec"],         domain: "binary", description: "Binary security checker", install: "pip install checksec" },
    ToolDef { name: "strings",    binaries: &["strings"],          domain: "binary", description: "Extract strings from binaries", install: "pre-installed" },
    ToolDef { name: "objdump",    binaries: &["objdump"],          domain: "binary", description: "Binary disassembler", install: "pre-installed" },
];

fn find_binary(binaries: &[&str]) -> Option<String> {
    for b in binaries {
        if which::which(b).is_ok() {
            return Some(b.to_string());
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_tools(state: State<'_, Arc<AppState>>) -> Result<Vec<ToolStatus>, String> {
    let tools: Vec<ToolStatus> = ALL_TOOLS
        .iter()
        .map(|t| {
            let bin = find_binary(t.binaries);
            ToolStatus {
                name:      t.name.to_string(),
                installed: bin.is_some(),
                binary:    bin.unwrap_or_default(),
                domain:    t.domain.to_string(),
                version:   String::new(),
            }
        })
        .collect();
    Ok(tools)
}

#[tauri::command]
async fn start_scan(
    target:   String,
    mode:     String,
    tools:    Vec<String>,
    app:      AppHandle,
    state:    State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let session = Session::new(&target, &mode);
    let session_id = session.id.clone();

    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.insert(session_id.clone(), session.clone());
        let mut active = state.active_scan.lock().unwrap();
        *active = Some(session_id.clone());
    }

    // Save to DB

