use crate::core::db::init_schema;
use crate::core::config::ensure_config_file;
use crate::core::types::{Session, Finding};
use anyhow::Result;
use rusqlite::Connection;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

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

pub struct AppState {
    pub db: Mutex<Connection>,
    pub sessions: Mutex<HashMap<String, Session>>,
    pub active_scan: Mutex<Option<String>>,
    pub config_path: PathBuf,
    pub reports_dir: PathBuf,
}

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".torot")
}

pub fn get_findings_internal(session_id: &str, state: &AppState) -> Vec<Finding> {
    let db = state.db.lock().unwrap();
    let mut stmt = match db.prepare(
        "SELECT id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp \
         FROM findings WHERE session_id=?1 \
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

pub fn get_db_stats(state: &AppState) -> serde_json::Value {
    let db = state.db.lock().unwrap();
    let sessions: i64 = db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap_or(0);
    let findings: i64 = db.query_row("SELECT COUNT(*) FROM findings", [], |r| r.get(0)).unwrap_or(0);
    let critical: i64 = db.query_row("SELECT COUNT(*) FROM findings WHERE severity='CRITICAL'", [], |r| r.get(0)).unwrap_or(0);
    let high: i64 = db.query_row("SELECT COUNT(*) FROM findings WHERE severity='HIGH'", [], |r| r.get(0)).unwrap_or(0);
    serde_json::json!({ "sessions": sessions, "findings": findings, "critical": critical, "high": high })
}

pub fn get_sessions(state: &AppState) -> Result<Vec<crate::core::types::DbSession>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id,target,domain,start_time,end_time,total_findings,summary FROM sessions ORDER BY start_time DESC LIMIT 100")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(crate::core::types::DbSession {
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

pub fn load_session_from_db(state: &AppState, session_id: &str) -> Result<Option<crate::core::types::Session>> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id,target,domain,start_time FROM sessions WHERE id=?1 LIMIT 1")?;
    let mut rows = stmt.query([session_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(crate::core::types::Session {
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

pub fn get_findings(session_id: String, state: &AppState) -> Vec<Finding> {
    get_findings_internal(&session_id, state)
}

pub fn stop_scan(state: &AppState) {
    *state.active_scan.lock().unwrap() = None;
}
