use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio::process::Command as TokioCommand;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;
use rusqlite::{Connection, params};

// ── Data Models ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String, pub session_id: String, pub tool: String,
    pub title: String, pub severity: String, pub domain: String,
    pub description: String, pub file: String, pub line: u32,
    pub code_snippet: String, pub fix_suggestion: String,
    pub impact: String, pub bug_type: String, pub timestamp: u64,
}

impl Finding {
    pub fn new(session_id: &str, tool: &str, title: &str, severity: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(), session_id: session_id.to_string(),
            tool: tool.to_string(), title: title.to_string(), severity: severity.to_string(),
            domain: String::new(), description: String::new(), file: String::new(),
            line: 0, code_snippet: String::new(), fix_suggestion: String::new(),
            impact: String::new(), bug_type: String::new(), timestamp: now_unix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String, pub target: String, pub mode: String,
    pub start_time: u64, pub findings: Vec<Finding>,
}
impl Session {
    pub fn new(target: &str, mode: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..12].to_string(),
            target: target.to_string(), mode: mode.to_string(),
            start_time: now_unix(), findings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus { pub name: String, pub installed: bool, pub binary: String, pub domain: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamLine {
    pub session_id: String, pub tool: String,
    pub line: String, pub kind: String, pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSession {
    pub id: String, pub target: String, pub domain: String,
    pub start_time: u64, pub end_time: u64, pub total_findings: u32, pub summary: String,
}

// ── App State ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Mutex<Connection>,
    pub sessions: Mutex<HashMap<String, Session>>,
    pub active_scan: Mutex<Option<String>>,
}
impl AppState {
    pub fn new() -> Result<Self> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir)?;
        let conn = Connection::open(dir.join("memory.db"))?;
        init_schema(&conn)?;
        Ok(Self { db: Mutex::new(conn), sessions: Mutex::new(HashMap::new()), active_scan: Mutex::new(None) })
    }
}

fn data_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".torot")
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, target TEXT, domain TEXT, start_time INTEGER, end_time INTEGER, total_findings INTEGER DEFAULT 0, summary TEXT);
        CREATE TABLE IF NOT EXISTS findings (id TEXT PRIMARY KEY, session_id TEXT, tool TEXT, title TEXT, severity TEXT, domain TEXT, description TEXT, file TEXT, line INTEGER, code_snippet TEXT, fix_suggestion TEXT, impact TEXT, bug_type TEXT, timestamp INTEGER);
        CREATE TABLE IF NOT EXISTS knowledge (id INTEGER PRIMARY KEY AUTOINCREMENT, topic TEXT, content TEXT, source TEXT, added_at INTEGER);
    ")?;
    Ok(())
}

fn now_unix() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs() }

// ── Tool Registry ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ToolDef { name: &'static str, binaries: &'static [&'static str], domain: &'static str }

static ALL_TOOLS: &[ToolDef] = &[
    ToolDef { name: "slither",    binaries: &["slither"],               domain: "blockchain" },
    ToolDef { name: "aderyn",     binaries: &["aderyn"],                domain: "blockchain" },
    ToolDef { name: "mythril",    binaries: &["myth"],                  domain: "blockchain" },
    ToolDef { name: "echidna",    binaries: &["echidna","echidna-test"],domain: "blockchain" },
    ToolDef { name: "manticore",  binaries: &["manticore"],             domain: "blockchain" },
    ToolDef { name: "solhint",    binaries: &["solhint"],               domain: "blockchain" },
    ToolDef { name: "halmos",     binaries: &["halmos"],                domain: "blockchain" },
    ToolDef { name: "semgrep",    binaries: &["semgrep"],               domain: "blockchain" },
    ToolDef { name: "solc",       binaries: &["solc"],                  domain: "blockchain" },
    ToolDef { name: "wake",       binaries: &["wake"],                  domain: "blockchain" },
    ToolDef { name: "heimdall",   binaries: &["heimdall"],              domain: "blockchain" },
    ToolDef { name: "cargo-audit",binaries: &["cargo-audit"],           domain: "blockchain" },
    ToolDef { name: "clippy",     binaries: &["cargo"],                 domain: "blockchain" },
    ToolDef { name: "nuclei",     binaries: &["nuclei"],                domain: "webapp" },
    ToolDef { name: "nikto",      binaries: &["nikto"],                 domain: "webapp" },
    ToolDef { name: "sqlmap",     binaries: &["sqlmap"],                domain: "webapp" },
    ToolDef { name: "ffuf",       binaries: &["ffuf"],                  domain: "webapp" },
    ToolDef { name: "gobuster",   binaries: &["gobuster"],              domain: "webapp" },
    ToolDef { name: "dalfox",     binaries: &["dalfox"],                domain: "webapp" },
    ToolDef { name: "trufflehog", binaries: &["trufflehog"],            domain: "webapp" },
    ToolDef { name: "gitleaks",   binaries: &["gitleaks"],              domain: "webapp" },
    ToolDef { name: "arjun",      binaries: &["arjun"],                 domain: "api" },
    ToolDef { name: "jwt_tool",   binaries: &["jwt_tool","jwt-tool"],   domain: "api" },
    ToolDef { name: "radare2",    binaries: &["r2"],                    domain: "binary" },
    ToolDef { name: "binwalk",    binaries: &["binwalk"],               domain: "binary" },
    ToolDef { name: "checksec",   binaries: &["checksec"],              domain: "binary" },
    ToolDef { name: "strings",    binaries: &["strings"],               domain: "binary" },
    ToolDef { name: "objdump",    binaries: &["objdump"],               domain: "binary" },
];

fn find_binary(bins: &[&str]) -> Option<String> {
    bins.iter().find(|b| which::which(b).is_ok()).map(|b| b.to_string())
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_tools(_state: State<'_, Arc<AppState>>) -> Result<Vec<ToolStatus>, String> {
    Ok(ALL_TOOLS.iter().map(|t| {
        let bin = find_binary(t.binaries);
        ToolStatus { name: t.name.to_string(), installed: bin.is_some(), binary: bin.unwrap_or_default(), domain: t.domain.to_string() }
    }).collect())
}

#[tauri::command]
async fn start_scan(target: String, mode: String, tools: Vec<String>, app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let session = Session::new(&target, &mode);
    let sid = session.id.clone();
    { state.sessions.lock().unwrap().insert(sid.clone(), session.clone()); *state.active_scan.lock().unwrap() = Some(sid.clone()); }
    { state.db.lock().unwrap().execute("INSERT OR REPLACE INTO sessions (id,target,domain,start_time,end_time,total_findings,summary) VALUES (?1,?2,'auto',?3,0,0,'')", params![&sid, &target, session.start_time]).ok(); }
    emit_line(&app, &sid, "system", "torot", &format!("Session {} | target: {} | mode: {} | tools: {}", &sid, &target, &mode, tools.join(", ")), None);
    let (sc, ac, ap, tg, tls) = (Arc::clone(&state), Arc::clone(&state), app.clone(), target.clone(), tools.clone());
    tokio::spawn(async move { run_pipeline(sid.clone(), tg, tls, ap, sc).await; });
    Ok(session.id)
}

async fn run_pipeline(sid: String, target: String, tools: Vec<String>, app: AppHandle, state: Arc<AppState>) {
    emit_line(&app, &sid, "system", "torot", &format!("Launching {} tool(s) in parallel...", tools.len()), None);
    let mut handles = Vec::new();
    for tn in &tools {
        let td = match ALL_TOOLS.iter().find(|t| t.name == tn) { Some(t) => t.clone(), None => continue };
        let bin = match find_binary(td.binaries) {
            Some(b) => b,
            None => { emit_line(&app, &sid, "system", td.name, &format!("[{}] not installed — skipped", td.name), None); continue; }
        };
        let (s2, a2, sid2, tgt2, tname) = (Arc::clone(&state), app.clone(), sid.clone(), target.clone(), td.name.to_string());
        handles.push(tokio::spawn(async move { run_tool(&sid2, &tgt2, &tname, &bin, a2, s2).await; }));
    }
    for h in handles { let _ = h.await; }
    let ts = now_unix();
    let total = { let sessions = state.sessions.lock().unwrap(); sessions.get(&sid).map(|s| s.findings.len()).unwrap_or(0) };
    { let db = state.db.lock().unwrap(); db.execute("UPDATE sessions SET end_time=?1, total_findings=?2 WHERE id=?3", params![ts, total as u32, &sid]).ok(); }
    emit_line(&app, &sid, "system", "torot", &format!("Scan complete — {} finding(s) total.", total), None);
    app.emit("scan_complete", serde_json::json!({ "session_id": sid, "total": total })).ok();
}

async fn run_tool(sid: &str, target: &str, tool_name: &str, binary: &str, app: AppHandle, state: Arc<AppState>) {
    emit_line(&app, sid, "system", tool_name, &format!("[{}] starting...", tool_name), None);
    let args = match build_args(tool_name, target) { Some(a) => a, None => { emit_line(&app, sid, "system", tool_name, &format!("[{}] no applicable target — skipped", tool_name), None); return; } };
    let cwd = if std::path::Path::new(target).is_dir() { target } else { "." };
    let mut child = match TokioCommand::new(binary).args(&args).current_dir(cwd).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => { emit_line(&app, sid, "system", tool_name, &format!("[{}] launch error: {}", tool_name, e), None); return; }
    };
    let mut all_out: Vec<String> = Vec::new();
    if let Some(so) = child.stdout.take() { let mut lines = BufReader::new(so).lines(); while let Ok(Some(l)) = lines.next_line().await { if !l.trim().is_empty() { emit_line(&app, sid, "output", tool_name, &l, None); all_out.push(l); } } }
    if let Some(se) = child.stderr.take() { let mut lines = BufReader::new(se).lines(); while let Ok(Some(l)) = lines.next_line().await { if !l.trim().is_empty() { emit_line(&app, sid, "output", tool_name, &l, None); all_out.push(l); } } }
    let _ = child.wait().await;
    let combined = all_out.join("\n");
    let findings = parse_output(sid, tool_name, &combined);
    let fcount = findings.len();
    for f in &findings {
        emit_line(&app, sid, "finding", tool_name, &f.title, Some(f.severity.clone()));
        { let db = state.db.lock().unwrap(); db.execute("INSERT OR IGNORE INTO findings (id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)", params![&f.id,&f.session_id,&f.tool,&f.title,&f.severity,&f.domain,&f.description,&f.file,f.line,&f.code_snippet,&f.fix_suggestion,&f.impact,&f.bug_type,f.timestamp]).ok(); }
        { let mut sessions = state.sessions.lock().unwrap(); if let Some(s) = sessions.get_mut(sid) { s.findings.push(f.clone()); } }
        app.emit("new_finding", f).ok();
    }
    emit_line(&app, sid, "system", tool_name, &format!("[{}] done — {} finding(s)", tool_name, fcount), None);
}

fn build_args(tool: &str, target: &str) -> Option<Vec<String>> {
    use std::path::Path;
    let is_dir  = Path::new(target).is_dir();
    let is_url  = target.starts_with("http://") || target.starts_with("https://");
    let is_file = Path::new(target).is_file();
    let first_sol: Option<String> = if is_dir { glob::glob(&format!("{}/**/*.sol", target)).ok().and_then(|mut g| g.next()).and_then(|p| p.ok()).map(|p| p.to_string_lossy().to_string()) } else if is_file && target.ends_with(".sol") { Some(target.to_string()) } else { None };
    let has_rs: bool = if is_dir { glob::glob(&format!("{}/**/*.rs", target)).ok().map(|mut g| g.next().is_some()).unwrap_or(false) } else { is_file && target.ends_with(".rs") };
    match tool {
        "slither"    => if first_sol.is_some() || is_dir { Some(vec![target.to_string(), "--json".into(), "-".into(), "--no-fail-pedantic".into()]) } else { None },
        "aderyn"     => if first_sol.is_some() || is_dir { Some(vec![target.to_string(), "--output".into(), "json".into()]) } else { None },
        "mythril"    => first_sol.map(|s| vec!["analyze".into(), s, "-o".into(), "json".into(), "--execution-timeout".into(), "60".into()]),
        "echidna"    => first_sol.map(|s| vec![s, "--format".into(), "text".into(), "--test-limit".into(), "1000".into()]),
        "manticore"  => first_sol.map(|s| vec![s]),
        "solhint"    => first_sol.map(|_| vec![format!("{}/**/*.sol", target), "--formatter".into(), "json".into()]),
        "halmos"     => if first_sol.is_some() || is_dir { Some(vec!["--root".into(), target.to_string(), "--json".into()]) } else { None },
        "solc"       => first_sol.map(|s| vec![s, "--combined-json".into(), "abi".into(), "--no-color".into()]),
        "wake"       => if first_sol.is_some() || is_dir { Some(vec!["detect".into(), "--json".into(), target.to_string()]) } else { None },
        "heimdall"   => first_sol.map(|s| vec!["decompile".into(), s]),
        "cargo-audit"=> if has_rs || is_dir { Some(vec!["audit".into(), "--json".into()]) } else { None },
        "clippy"     => if has_rs || is_dir { Some(vec!["clippy".into(), "--message-format=json".into(), "--".into(), "-D".into(), "warnings".into()]) } else { None },
        "semgrep"    => Some(vec!["--config".into(), "auto".into(), "--json".into(), target.to_string(), "--quiet".into()]),
        "nuclei"     => if is_url { Some(vec!["-target".into(), target.to_string(), "-json".into(), "-silent".into()]) } else { None },
        "nikto"      => if is_url { Some(vec!["-h".into(), target.to_string(), "-Format".into(), "txt".into()]) } else { None },
        "sqlmap"     => if is_url { Some(vec!["-u".into(), target.to_string(), "--batch".into(), "--level=2".into()]) } else { None },
        "ffuf"       => if is_url { Some(vec!["-u".into(), format!("{}/FUZZ", target), "-w".into(), "/usr/share/wordlists/dirb/common.txt".into(), "-o".into(), "json".into()]) } else { None },
        "gobuster"   => if is_url { Some(vec!["dir".into(), "-u".into(), target.to_string(), "-w".into(), "/usr/share/wordlists/dirb/common.txt".into()]) } else { None },
        "dalfox"     => if is_url { Some(vec!["url".into(), target.to_string(), "--silence".into()]) } else { None },
        "trufflehog" => Some(vec!["filesystem".into(), target.to_string(), "--json".into()]),
        "gitleaks"   => Some(vec!["detect".into(), "--source".into(), target.to_string(), "--report-format".into(), "json".into()]),
        "arjun"      => if is_url { Some(vec!["-u".into(), target.to_string(), "--json".into()]) } else { None },
        "jwt_tool"|"jwt-tool" => if is_url { Some(vec![target.to_string(), "-t".into()]) } else { None },
        "radare2"    => if is_file { Some(vec!["-A".into(), "-q".into(), "-c".into(), "aaa; pdf @ main".into(), target.to_string()]) } else { None },
        "binwalk"    => if is_file { Some(vec!["-e".into(), target.to_string()]) } else { None },
        "checksec"   => if is_file { Some(vec!["--file".into(), target.to_string(), "--output".into(), "json".into()]) } else { None },
        "strings"    => if is_file { Some(vec![target.to_string()]) } else { None },
        "objdump"    => if is_file { Some(vec!["-d".into(), target.to_string()]) } else { None },
        _            => Some(vec![target.to_string()]),
    }
}

fn sev_from_text(t: &str) -> &'static str {
    let l = t.to_lowercase();
    if l.contains("critical") { "CRITICAL" } else if l.contains("high") || l.contains("error:") { "HIGH" } else if l.contains("medium") || l.contains("warning") { "MEDIUM" } else if l.contains("low") { "LOW" } else { "INFO" }
}

fn domain_of(tool: &str) -> &'static str {
    match tool { "slither"|"aderyn"|"mythril"|"echidna"|"manticore"|"solhint"|"halmos"|"solc"|"wake"|"heimdall"|"cargo-audit"|"clippy" => "blockchain", "nuclei"|"nikto"|"sqlmap"|"ffuf"|"gobuster"|"dalfox"|"trufflehog"|"gitleaks" => "webapp", "arjun"|"jwt_tool" => "api", "radare2"|"binwalk"|"checksec"|"strings"|"objdump" => "binary", _ => "general" }
}

fn parse_output(sid: &str, tool: &str, output: &str) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    // Try whole-document JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) { out.extend(parse_json(sid, tool, &json)); }
    // Try NDJSON
    if out.is_empty() { for line in output.lines() { let l = line.trim(); if (l.starts_with('{') || l.starts_with('[')) { if let Ok(j) = serde_json::from_str::<serde_json::Value>(l) { out.extend(parse_json(sid, tool, &j)); } } } }
    // Fallback text
    if out.is_empty() { out.extend(parse_text(sid, tool, output)); }
    out.dedup_by(|a, b| a.description == b.description);
    out.sort_by(|a, b| { let o = |s: &str| match s { "CRITICAL"=>0,"HIGH"=>1,"MEDIUM"=>2,"LOW"=>3,_=>4 }; o(&a.severity).cmp(&o(&b.severity)) });
    out
}

fn parse_json(sid: &str, tool: &str, json: &serde_json::Value) -> Vec<Finding> {
    let mut out = Vec::new();
    match tool {
        "slither" => { if let Some(dets) = json.pointer("/results/detectors").and_then(|v| v.as_array()) { for det in dets { let check = det["check"].as_str().unwrap_or("issue"); let sev = match det["impact"].as_str().unwrap_or("Low") { "High"=>"HIGH","Medium"=>"MEDIUM","Low"=>"LOW",_=>"INFO" }; let mut f = Finding::new(sid, tool, &format!("[slither] {}", check.replace('-',' ')), sev); f.description = det["description"].as_str().unwrap_or("").to_string(); f.domain = "blockchain".to_string(); f.bug_type = check.to_string(); out.push(f); } } }
        "mythril" => { if let Some(issues) = json["issues"].as_array() { for issue in issues { let sev = match issue["severity"].as_str().unwrap_or("Low") { "High"=>"HIGH","Medium"=>"MEDIUM",_=>"LOW" }; let mut f = Finding::new(sid, tool, &format!("[mythril] {}", issue["title"].as_str().unwrap_or("Issue")), sev); f.description = issue["description"].as_str().unwrap_or("").to_string(); f.file = issue["filename"].as_str().unwrap_or("").to_string(); f.line = issue["lineno"].as_u64().unwrap_or(0) as u32; f.domain = "blockchain".to_string(); out.push(f); } } }
        "semgrep" => { if let Some(results) = json["results"].as_array() { for r in results { let sev = match r.pointer("/extra/severity").and_then(|v| v.as_str()).unwrap_or("WARNING") { "ERROR"=>"HIGH","WARNING"=>"MEDIUM",_=>"LOW" }; let cid = r["check_id"].as_str().unwrap_or("rule"); let mut f = Finding::new(sid, tool, &format!("[semgrep] {}", cid.split('.').last().unwrap_or(cid)), sev); f.description = r.pointer("/extra/message").and_then(|v| v.as_str()).unwrap_or("").to_string(); f.file = r["path"].as_str().unwrap_or("").to_string(); f.line = r.pointer("/start/line").and_then(|v| v.as_u64()).unwrap_or(0) as u32; out.push(f); } } }
        "nuclei" => { let sev = match json.pointer("/info/severity").and_then(|v| v.as_str()).unwrap_or("info") { "critical"=>"CRITICAL","high"=>"HIGH","medium"=>"MEDIUM","low"=>"LOW",_=>"INFO" }; let name = json.pointer("/info/name").and_then(|v| v.as_str()).unwrap_or("finding"); let mut f = Finding::new(sid, tool, &format!("[nuclei] {}", name), sev); f.description = json.pointer("/info/description").and_then(|v| v.as_str()).unwrap_or("").to_string(); f.file = json["matched-at"].as_str().unwrap_or("").to_string(); f.domain = "webapp".to_string(); if !f.file.is_empty() { out.push(f); } }
        _ => {}
    }
    out
}

fn parse_text(sid: &str, tool: &str, output: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let kws = ["reentrancy","overflow","selfdestruct","tx.origin","vulnerability","critical","injection","xss","sqli","ssrf","rce","lfi","idor","buffer overflow","use-after-free","error:","FAILED","assertion failed","violation"];
    let domain = domain_of(tool);
    for line in output.lines() {
        let lw = line.to_lowercase();
        if kws.iter().any(|k| lw.contains(k)) && line.trim().len() > 15 {
            let sev = sev_from_text(line);
            let mut f = Finding::new(sid, tool, &format!("[{}] {}", tool, &line.trim()[..line.trim().len().min(80)]), sev);
            f.description = line.trim().to_string();
            f.domain = domain.to_string();
            out.push(f);
        }
    }
    out
}

fn emit_line(app: &AppHandle, sid: &str, kind: &str, tool: &str, line: &str, severity: Option<String>) {
    app.emit("stream_line", StreamLine { session_id: sid.to_string(), tool: tool.to_string(), line: line.to_string(), kind: kind.to_string(), severity }).ok();
}

// ── DB Queries ───────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_sessions(state: State<'_, Arc<AppState>>) -> Result<Vec<DbSession>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id,target,domain,start_time,end_time,total_findings,summary FROM sessions ORDER BY start_time DESC LIMIT 100").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(DbSession { id: r.get(0)?, target: r.get(1)?, domain: r.get(2)?, start_time: r.get(3)?, end_time: r.get(4)?, total_findings: r.get(5)?, summary: r.get(6)? })).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
async fn get_findings(session_id: String, state: State<'_, Arc<AppState>>) -> Result<Vec<Finding>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp FROM findings WHERE session_id=?1 ORDER BY CASE severity WHEN 'CRITICAL' THEN 0 WHEN 'HIGH' THEN 1 WHEN 'MEDIUM' THEN 2 WHEN 'LOW' THEN 3 ELSE 4 END").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([&session_id], |r| Ok(Finding { id: r.get(0)?, session_id: r.get(1)?, tool: r.get(2)?, title: r.get(3)?, severity: r.get(4)?, domain: r.get(5)?, description: r.get(6)?, file: r.get(7)?, line: r.get(8)?, code_snippet: r.get(9)?, fix_suggestion: r.get(10)?, impact: r.get(11)?, bug_type: r.get(12)?, timestamp: r.get(13)? })).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
async fn stop_scan(state: State<'_, Arc<AppState>>) -> Result<(), String> { *state.active_scan.lock().unwrap() = None; Ok(()) }

#[tauri::command]
async fn get_db_stats(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().unwrap();
    let (sessions, findings, critical, high): (i64,i64,i64,i64) = (
        db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap_or(0),
        db.query_row("SELECT COUNT(*) FROM findings",  [], |r| r.get(0)).unwrap_or(0),
        db.query_row("SELECT COUNT(*) FROM findings WHERE severity='CRITICAL'", [], |r| r.get(0)).unwrap_or(0),
        db.query_row("SELECT COUNT(*) FROM findings WHERE severity='HIGH'",     [], |r| r.get(0)).unwrap_or(0),
    );
    Ok(serde_json::json!({ "sessions": sessions, "findings": findings, "critical": critical, "high": high }))
}

// ── Entry ────────────────────────────────────────────────────────────────────

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
        .invoke_handler(tauri::generate_handler![get_tools, start_scan, stop_scan, get_sessions, get_findings, get_db_stats])
        .run(tauri::generate_context!())
        .expect("error running Torot");
}
