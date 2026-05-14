use anyhow::Result;
use rusqlite::Connection;

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

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
            timestamp INTEGER,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE INDEX IF NOT EXISTS idx_findings_session ON findings(session_id);
        CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
        CREATE INDEX IF NOT EXISTS idx_sessions_start ON sessions(start_time);
        ",
    )?;
    Ok(())
}

pub fn insert_session(db: &Connection, id: &str, target: &str, domain: &str, start_time: u64) {
    let _ = db.execute(
        "INSERT OR REPLACE INTO sessions (id,target,domain,start_time,end_time,total_findings,summary) VALUES (?1,?2,?3,?4,0,0,'')",
        rusqlite::params![id, target, domain, start_time],
    );
}

pub fn update_session(db: &Connection, id: &str, end_time: u64, total: u32, summary: &str) {
    let _ = db.execute(
        "UPDATE sessions SET end_time=?1, total_findings=?2, summary=?3 WHERE id=?4",
        rusqlite::params![end_time, total, summary, id],
    );
}

pub fn insert_finding(db: &Connection, f: &crate::core::types::Finding) {
    let _ = db.execute(
        "INSERT OR IGNORE INTO findings (id,session_id,tool,title,severity,domain,description,file,line,code_snippet,fix_suggestion,impact,bug_type,timestamp) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        rusqlite::params![
            &f.id, &f.session_id, &f.tool, &f.title, &f.severity,
            &f.domain, &f.description, &f.file, f.line, &f.code_snippet,
            &f.fix_suggestion, &f.impact, &f.bug_type, f.timestamp,
        ],
    );
}

pub fn count_sessions(db: &Connection) -> i64 {
    db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap_or(0)
}

pub fn count_findings(db: &Connection) -> i64 {
    db.query_row("SELECT COUNT(*) FROM findings", [], |r| r.get(0)).unwrap_or(0)
}

pub fn count_critical(db: &Connection) -> i64 {
    db.query_row("SELECT COUNT(*) FROM findings WHERE severity='CRITICAL'", [], |r| r.get(0)).unwrap_or(0)
}

pub fn count_high(db: &Connection) -> i64 {
    db.query_row("SELECT COUNT(*) FROM findings WHERE severity='HIGH'", [], |r| r.get(0)).unwrap_or(0)
}
