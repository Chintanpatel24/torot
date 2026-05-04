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
