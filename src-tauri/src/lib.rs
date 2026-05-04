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
   {
        let db = state.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO sessions (id, target, domain, start_time, end_time, total_findings, summary)
             VALUES (?1, ?2, ?3, ?4, 0, 0, '')",
            params![&session_id, &target, "auto", session.start_time],
        ).ok();
    }

    emit_line(&app, &session_id, "system", "torot", &format!("Session {} started", &session_id), None);
    emit_line(&app, &session_id, "system", "torot", &format!("Target: {}", &target), None);
    emit_line(&app, &session_id, "system", "torot", &format!("Mode: {}", &mode), None);
    emit_line(&app, &session_id, "system", "torot", &format!("Tools selected: {}", tools.join(", ")), None);

    let state_clone = Arc::clone(&state);
    let app_clone   = app.clone();
    let sid         = session_id.clone();
    let tgt         = target.clone();

    tokio::spawn(async move {
        run_scan_pipeline(sid, tgt, tools, app_clone, state_clone).await;
    });

    Ok(session_id)
}

async fn run_scan_pipeline(
    session_id: String,
    target:     String,
    tools:      Vec<String>,
    app:        AppHandle,
    state:      Arc<AppState>,
) {
    emit_line(&app, &session_id, "system", "torot", "Starting parallel tool execution...", None);

    let mut handles = Vec::new();

    for tool_name in &tools {
        let tool_def = ALL_TOOLS.iter().find(|t| t.name == tool_name);
        if tool_def.is_none() { continue; }
        let td = tool_def.unwrap().clone();
        let binary = match find_binary(td.binaries) {
            Some(b) => b,
            None => {
                emit_line(&app, &session_id, "system", &td.name, &format!("{} not installed — skipping", td.name), None);
                continue;
            }
        };

        let sid2    = session_id.clone();
        let tgt2    = target.clone();
        let app2    = app.clone();
        let state2  = Arc::clone(&state);
        let tname   = td.name.to_string();

        let handle = tokio::spawn(async move {
            run_single_tool(&sid2, &tgt2, &tname, &binary, app2, state2).await
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    // Finalize session
    let ts = now_unix();
    {
        let db = state.db.lock().unwrap();
        let sessions = state.sessions.lock().unwrap();
        if let Some(sess) = sessions.get(&session_id) {
            let count = sess.findings.len() as u32;
            let summary = serde_json::to_string(&sess.findings.iter().fold(
                HashMap::<String,u32>::new(),
                |mut m, f| { *m.entry(f.severity.clone()).or_insert(0) += 1; m }
            )).unwrap_or_default();
            db.execute(
                "UPDATE sessions SET end_time=?1, total_findings=?2, summary=?3 WHERE id=?4",
                params![ts, count, summary, &session_id],
            ).ok();
        }
    }

    let total = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&session_id).map(|s| s.findings.len()).unwrap_or(0)
    };

    emit_line(&app, &session_id, "system", "torot",
        &format!("Scan complete. {} findings total.", total), None);
    app.emit("scan_complete", serde_json::json!({ "session_id": session_id, "total": total })).ok();
}

async fn run_single_tool(
    session_id: &str,
    target:     &str,
    tool_name:  &str,
    binary:     &str,
    app:        AppHandle,
    state:      Arc<AppState>,
) {
    emit_line(&app, session_id, "system", tool_name, &format!("[{}] Starting...", tool_name), None);

    let args = build_args(tool_name, binary, target);
    if args.is_empty() {
        emit_line(&app, session_id, "system", tool_name,
            &format!("[{}] No applicable target found", tool_name), None);
        return;
    }

    let cwd = if std::path::Path::new(target).is_dir() { target } else { "." };

    let mut child = match Command::new(binary)
        .args(&args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c)  => c,
        Err(e) => {
            emit_line(&app, session_id, "system", tool_name,
                &format!("[{}] Failed to start: {}", tool_name, e), None);
            return;
        }
    };

    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    let mut all_output = Vec::<String>::new();

    if let Some(mut reader) = stdout {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            emit_line(&app, session_id, "output", tool_name, &line, None);
            all_output.push(line);
        }
    }
    if let Some(mut reader) = stderr {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.is_empty() {
                emit_line(&app, session_id, "output", tool_name, &line, None);
                all_output.push(line);
            }
        }
    }

    let _ = child.wait().await;

    let combined = all_output.join("\n");
    let findings = parse_output(session_id, tool_name, &combined);

    for f in &findings {
        emit_line(&app, session_id, "finding", tool_name, &f.title,
            Some(f.severity.clone()));
        // save to DB
        let db = state.db.lock().unwrap();
        db.execute(
            "INSERT OR IGNORE INTO findings (id, session_id, tool, title, severity, domain, description, file, line, code_snippet, fix_suggestion, impact, bug_type, timestamp)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                &f.id, &f.session_id, &f.tool, &f.title, &f.severity,
                &f.domain, &f.description, &f.file, f.line,
                &f.code_snippet, &f.fix_suggestion, &f.impact,
                &f.bug_type, f.timestamp
            ],
        ).ok();
        drop(db);

        let mut sessions = state.sessions.lock().unwrap();
        if let Some(sess) = sessions.get_mut(session_id) {
            sess.findings.push(f.clone());
        }

        app.emit("new_finding", f).ok();
    }

    emit_line(&app, session_id, "system", tool_name,
        &format!("[{}] Done. {} findings.", tool_name, findings.len()), None);
}

fn build_args(tool: &str, binary: &str, target: &str) -> Vec<String> {
    use std::path::Path;
    let is_dir = Path::new(target).is_dir();

    let sol_glob = if is_dir {
        glob::glob(&format!("{}/**/*.sol", target))
            .ok().and_then(|mut g| g.next())
            .and_then(|p| p.ok())
            .map(|p| p.to_string_lossy().to_string())
    } else { None };

    match tool {
        "slither"     => vec![target.to_string(), "--json".to_string(), "-".to_string(), "--no-fail-pedantic".to_string()],
        "aderyn"      => vec![target.to_string(), "--output".to_string(), "json".to_string()],
        "mythril"     => sol_glob.map(|s| vec!["analyze".to_string(), s, "-o".to_string(), "json".to_string(), "--execution-timeout".to_string(), "60".to_string()]).unwrap_or_default(),
        "echidna"     => sol_glob.map(|s| vec![s, "--format".to_string(), "text".to_string(), "--test-limit".to_string(), "1000".to_string()]).unwrap_or_default(),
        "semgrep"     => vec!["--config".to_string(), "auto".to_string(), "--json".to_string(), target.to_string(), "--quiet".to_string()],
        "solhint"     => vec![format!("{}/**/*.sol", target), "--formatter".to_string(), "json".to_string()],
        "nuclei"      => vec!["-target".to_string(), target.to_string(), "-json".to_string(), "-silent".to_string()],
        "nikto"       => vec!["-h".to_string(), target.to_string(), "-Format".to_string(), "txt".to_string()],
        "sqlmap"      => vec!["-u".to_string(), target.to_string(), "--batch".to_string(), "--level=2".to_string()],
        "ffuf"        => vec!["-u".to_string(), format!("{}/FUZZ", target), "-w".to_string(), "/usr/share/wordlists/common.txt".to_string(), "-o".to_string(), "json".to_string()],
        "gobuster"    => vec!["dir".to_string(), "-u".to_string(), target.to_string(), "-w".to_string(), "/usr/share/wordlists/dirb/common.txt".to_string()],
        "dalfox"      => vec!["url".to_string(), target.to_string(), "--silence".to_string()],
        "trufflehog"  => vec!["filesystem".to_string(), target.to_string(), "--json".to_string()],
        "gitleaks"    => vec!["detect".to_string(), "--source".to_string(), target.to_string(), "--report-format".to_string(), "json".to_string()],
        "checksec"    => vec!["--file".to_string(), target.to_string(), "--output".to_string(), "json".to_string()],
        "strings"     => vec![target.to_string()],
        "cargo-audit" => vec!["audit".to_string(), "--json".to_string()],
        "clippy"      => vec!["clippy".to_string(), "--message-format=json".to_string(), "--".to_string(), "-D".to_string(), "warnings".to_string()],
        _ => vec![target.to_string()],
    }
}

fn parse_output(session_id: &str, tool: &str, output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let keywords = ["error", "warning", "vulnerability", "critical", "high risk",
                    "medium risk", "low risk", "reentrancy", "overflow", "injection",
                    "xss", "sqli", "rce", "lfi", "ssrf", "idor", "failed", "violation"];

    for line in output.lines() {
        let lw = line.to_lowercase();
        if keywords.iter().any(|k| lw.contains(k)) && line.len() > 10 {
            let sev = if lw.contains("critical") || lw.contains("high risk") { "CRITICAL" }
                      else if lw.contains("high") || lw.contains("error") { "HIGH" }
                      else if lw.contains("medium") || lw.contains("warning") { "MEDIUM" }
                      else if lw.contains("low") { "LOW" }
                      else { "INFO" };

            let mut f = Finding::new(session_id, tool, &format!("[{}] {}", tool, &line[..line.len().min(80)]), sev);
            f.description = line.trim().to_string();
            f.domain = match tool {
                "slither"|"aderyn"|"mythril"|"echidna"|"halmos"|"solhint"|"wake" => "blockchain",
                "nuclei"|"nikto"|"sqlmap"|"ffuf"|"gobuster"|"dalfox" => "webapp",
                "radare2"|"binwalk"|"checksec"|"strings"|"objdump" => "binary",
                "arjun"|"jwt_tool" => "api",
                _ => "general",
            }.to_string();
            findings.push(f);
        }
    }

   // JSON parsing for tools that output JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        findings.extend(parse_json_output(session_id, tool, &json));
    } else {
        // Try line-by-line JSON (ndjson)
        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                findings.extend(parse_json_output(session_id, tool, &json));
            }
        }
    }

    findings.sort_by(|a, b| {
        let order = |s: &str| match s { "CRITICAL"=>0,"HIGH"=>1,"MEDIUM"=>2,"LOW"=>3,_=>4 };
        order(&a.severity).cmp(&order(&b.severity))
    });
    findings.dedup_by(|a, b| a.description == b.description);
    findings
}

fn parse_json_output(session_id: &str, tool: &str, json: &serde_json::Value) -> Vec<Finding> {
    let mut findings = Vec::new();
    match tool {
        "slither" => {
            if let Some(detectors) = json.pointer("/results/detectors").and_then(|v| v.as_array()) {
                for det in detectors {
                    let check = det["check"].as_str().unwrap_or("issue");
                    let impact = det["impact"].as_str().unwrap_or("Low");
                    let sev = match impact { "High" => "HIGH", "Medium" => "MEDIUM", "Low" => "LOW", _ => "INFO" };
                    let desc = det["description"].as_str().unwrap_or("").to_string();
                    let mut f = Finding::new(session_id, tool, &format!("[slither] {}", check.replace('-',' ')), sev);
                    f.description = desc;
                    f.domain = "blockchain".to_string();
                    f.bug_type = check.to_string();
                    findings.push(f);
                }
            }
        }
        "mythril" => {
            if let Some(issues) = json["issues"].as_array() {
                for issue in issues {
                    let title = issue["title"].as_str().unwrap_or("issue");
                    let sev = match issue["severity"].as_str().unwrap_or("Low") {
                        "High" => "HIGH", "Medium" => "MEDIUM", _ => "LOW"
                    };
                    let mut f = Finding::new(session_id, tool, &format!("[mythril] {}", title), sev);
                    f.description = issue["description"].as_str().unwrap_or("").to_string();
                    f.file = issue["filename"].as_str().unwrap_or("").to_string();
                    f.line = issue["lineno"].as_u64().unwrap_or(0) as u32;
                    f.domain = "blockchain".to_string();
                    findings.push(f);
                }
            }
        }
        "semgrep" => {
            if let Some(results) = json["results"].as_array() {
                for r in results {
                    let check_id = r["check_id"].as_str().unwrap_or("rule");
                    let sev_raw  = r["extra"]["severity"].as_str().unwrap_or("WARNING");
                    let sev = match sev_raw { "ERROR" => "HIGH", "WARNING" => "MEDIUM", _ => "LOW" };
                    let msg = r["extra"]["message"].as_str().unwrap_or("").to_string();
                    let mut f = Finding::new(session_id, tool, &format!("[semgrep] {}", check_id.split('.').last().unwrap_or(check_id)), sev);
                    f.description = msg;
                    f.file = r["path"].as_str().unwrap_or("").to_string();
                    f.line = r["start"]["line"].as_u64().unwrap_or(0) as u32;
                    findings.push(f);
                }
            }
        }
        "nuclei" => {
            let sev_raw = json["info"]["severity"].as_str().unwrap_or("info");
            let sev = match sev_raw { "critical" => "CRITICAL", "high" => "HIGH", "medium" => "MEDIUM", "low" => "LOW", _ => "INFO" };
            let name = json["info"]["name"].as_str().unwrap_or("nuclei finding");
            let mut f = Finding::new(session_id, tool, &format!("[nuclei] {}", name), sev);
            f.description = json["info"]["description"].as_str().unwrap_or("").to_string();
            f.file = json["matched-at"].as_str().unwrap_or("").to_string();
            f.domain = "webapp".to_string();
            if f.title.len() > 10 { findings.push(f); }
        }
        _ => {}
    }
    findings
}

fn emit_line(app: &AppHandle, session_id: &str, kind: &str, tool: &str, line: &str, severity: Option<String>) {
    app.emit("stream_line", StreamLine {
        session_id: session_id.to_string(),
        tool:       tool.to_string(),
        line:       line.to_string(),
        kind:       kind.to_string(),
        severity,
    }).ok();
}

#[tauri::command]
async fn get_sessions(state: State<'_, Arc<AppState>>) -> Result<Vec<DbSession>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT id, target, domain, start_time, end_time, total_findings, summary FROM sessions ORDER BY start_time DESC LIMIT 50"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(DbSession {
            id:             row.get(0)?,
            target:         row.get(1)?,
            domain:         row.get(2)?,
            start_time:     row.get(3)?,
            end_time:       row.get(4)?,
            total_findings: row.get(5)?,
            summary:        row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
async fn get_findings(session_id: String, state: State<'_, Arc<AppState>>) -> Result<Vec<Finding>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT id, session_id, tool, title, severity, domain, description, file, line, code_snippet, fix_suggestion, impact, bug_type, timestamp
         FROM findings WHERE session_id=?1 ORDER BY CASE severity WHEN 'CRITICAL' THEN 0 WHEN 'HIGH' THEN 1 WHEN 'MEDIUM' THEN 2 WHEN 'LOW' THEN 3 ELSE 4 END"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([&session_id], |row| {
        Ok(Finding {
            id:             row.get(0)?,
            session_id:     row.get(1)?,
            tool:           row.get(2)?,
            title:          row.get(3)?,
            severity:       row.get(4)?,
            domain:         row.get(5)?,
            description:    row.get(6)?,
            file:           row.get(7)?,
            line:           row.get(8)?,
            code_snippet:   row.get(9)?,
            fix_suggestion: row.get(10)?,
            impact:         row.get(11)?,
            bug_type:       row.get(12)?,
            timestamp:      row.get(13)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
async fn stop_scan(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut active = state.active_scan.lock().unwrap();
    *active = None;
    Ok(())
}

#[tauri::command]
async fn get_db_stats(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().unwrap();
    let sessions: i64 = db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap_or(0);
    let findings: i64 = db.query_row("SELECT COUNT(*) FROM findings",  [], |r| r.get(0)).unwrap_or(0);
    let critical: i64 = db.query_row("SELECT COUNT(*) FROM findings WHERE severity='CRITICAL'", [], |r| r.get(0)).unwrap_or(0);
    let high:     i64 = db.query_row("SELECT COUNT(*) FROM findings WHERE severity='HIGH'",     [], |r| r.get(0)).unwrap_or(0);
    Ok(serde_json::json!({ "sessions": sessions, "findings": findings, "critical": critical, "high": high }))
}

// ─────────────────────────────────────────────────────────────────────────────
// App Entry
// ─────────────────────────────────────────────────────────────────────────────

