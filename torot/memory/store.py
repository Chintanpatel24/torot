"""
Torot Memory System
- Session memory (in-process, fast)
- Persistent memory (SQLite, survives restarts)
- Exportable knowledge base (JSON/Markdown)
"""
from __future__ import annotations
import sqlite3
import json
import time
import os
from pathlib import Path
from typing import Optional
from torot.core.models import Session, Finding, ChatMessage, Severity


TOROT_DIR   = Path.home() / ".torot"
DB_PATH     = TOROT_DIR / "memory.db"
EXPORT_DIR  = TOROT_DIR / "exports"


def _ensure_dirs():
    TOROT_DIR.mkdir(exist_ok=True)
    EXPORT_DIR.mkdir(exist_ok=True)


def init_db():
    _ensure_dirs()
    conn = sqlite3.connect(str(DB_PATH))
    c    = conn.cursor()
    c.executescript("""
        CREATE TABLE IF NOT EXISTS sessions (
            id          TEXT PRIMARY KEY,
            target      TEXT,
            input_mode  TEXT,
            domain      TEXT,
            start_time  REAL,
            end_time    REAL,
            total_findings INTEGER DEFAULT 0,
            summary     TEXT
        );

        CREATE TABLE IF NOT EXISTS findings (
            id          TEXT PRIMARY KEY,
            session_id  TEXT,
            tool        TEXT,
            title       TEXT,
            severity    TEXT,
            domain      TEXT,
            description TEXT,
            file        TEXT,
            line        INTEGER,
            code_snippet TEXT,
            fix_suggestion TEXT,
            impact      TEXT,
            bug_type    TEXT,
            timestamp   REAL,
            FOREIGN KEY(session_id) REFERENCES sessions(id)
        );

        CREATE TABLE IF NOT EXISTS messages (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id  TEXT,
            role        TEXT,
            content     TEXT,
            tool_name   TEXT,
            timestamp   REAL,
            FOREIGN KEY(session_id) REFERENCES sessions(id)
        );

        CREATE TABLE IF NOT EXISTS knowledge (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            topic       TEXT,
            content     TEXT,
            source      TEXT,
            added_at    REAL
        );
    """)
    conn.commit()
    conn.close()


class MemoryStore:
    """
    Dual-layer memory:
      - session_cache: fast in-process dict for current session
      - SQLite: persistent across restarts
    """

    def __init__(self):
        _ensure_dirs()
        init_db()
        self._session_cache: dict[str, Session] = {}

    # ── Session ─────────────────────────────────────────────────────────

    def save_session(self, session: Session):
        self._session_cache[session.id] = session
        summary = json.dumps(session.finding_summary)
        conn = sqlite3.connect(str(DB_PATH))
        conn.execute("""
            INSERT OR REPLACE INTO sessions
            (id, target, input_mode, domain, start_time, end_time, total_findings, summary)
            VALUES (?,?,?,?,?,?,?,?)
        """, (
            session.id,
            session.target,
            session.input_mode.value,
            session.domain.value,
            session.start_time,
            session.end_time or time.time(),
            len(session.findings),
            summary,
        ))
        conn.commit()
        conn.close()

    def save_finding(self, session_id: str, finding: Finding):
        conn = sqlite3.connect(str(DB_PATH))
        conn.execute("""
            INSERT OR REPLACE INTO findings
            (id, session_id, tool, title, severity, domain, description,
             file, line, code_snippet, fix_suggestion, impact, bug_type, timestamp)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)
        """, (
            finding.id, session_id, finding.tool, finding.title,
            finding.severity.value, finding.domain.value, finding.description,
            finding.file, finding.line, finding.code_snippet,
            finding.fix_suggestion, finding.impact, finding.bug_type,
            finding.timestamp,
        ))
        conn.commit()
        conn.close()

    def save_message(self, session_id: str, msg: ChatMessage):
        conn = sqlite3.connect(str(DB_PATH))
        conn.execute("""
            INSERT INTO messages (session_id, role, content, tool_name, timestamp)
            VALUES (?,?,?,?,?)
        """, (session_id, msg.role, msg.content, msg.tool_name, msg.timestamp))
        conn.commit()
        conn.close()

    def get_recent_sessions(self, limit: int = 10) -> list[dict]:
        conn = sqlite3.connect(str(DB_PATH))
        rows = conn.execute("""
            SELECT id, target, domain, start_time, total_findings, summary
            FROM sessions ORDER BY start_time DESC LIMIT ?
        """, (limit,)).fetchall()
        conn.close()
        return [
            {"id": r[0], "target": r[1], "domain": r[2],
             "start_time": r[3], "total_findings": r[4], "summary": r[5]}
            for r in rows
        ]

    def get_all_findings(self, severity: Optional[str] = None, limit: int = 100) -> list[dict]:
        conn   = sqlite3.connect(str(DB_PATH))
        query  = "SELECT * FROM findings"
        params: list = []
        if severity:
            query += " WHERE severity=?"
            params.append(severity)
        query += " ORDER BY timestamp DESC LIMIT ?"
        params.append(limit)
        rows = conn.execute(query, params).fetchall()
        conn.close()
        cols = ["id","session_id","tool","title","severity","domain",
                "description","file","line","code_snippet","fix_suggestion",
                "impact","bug_type","timestamp"]
        return [dict(zip(cols, r)) for r in rows]

    def add_knowledge(self, topic: str, content: str, source: str = ""):
        conn = sqlite3.connect(str(DB_PATH))
        conn.execute(
            "INSERT INTO knowledge (topic, content, source, added_at) VALUES (?,?,?,?)",
            (topic, content, source, time.time())
        )
        conn.commit()
        conn.close()

    def search_knowledge(self, query: str) -> list[dict]:
        conn = sqlite3.connect(str(DB_PATH))
        rows = conn.execute("""
            SELECT topic, content, source, added_at
            FROM knowledge WHERE content LIKE ? OR topic LIKE ?
            ORDER BY added_at DESC LIMIT 10
        """, (f"%{query}%", f"%{query}%")).fetchall()
        conn.close()
        return [{"topic": r[0], "content": r[1], "source": r[2], "added_at": r[3]} for r in rows]

    # ── Export ──────────────────────────────────────────────────────────

    def export_session_json(self, session: Session) -> Path:
        _ensure_dirs()
        ts   = time.strftime("%Y%m%d_%H%M%S")
        path = EXPORT_DIR / f"session_{session.id}_{ts}.json"
        data = {
            "session_id":   session.id,
            "target":       session.target,
            "domain":       session.domain.value,
            "duration":     session.duration,
            "findings":     [
                {
                    "id": f.id, "tool": f.tool, "title": f.title,
                    "severity": f.severity.value, "file": f.file,
                    "line": f.line, "description": f.description,
                    "fix": f.fix_suggestion, "impact": f.impact,
                }
                for f in session.findings
            ],
            "summary":      session.finding_summary,
        }
        path.write_text(json.dumps(data, indent=2))
        return path

    def export_session_markdown(self, session: Session) -> Path:
        from torot.core.report import generate_report
        _ensure_dirs()
        ts   = time.strftime("%Y%m%d_%H%M%S")
        path = EXPORT_DIR / f"report_{session.id}_{ts}.md"
        generate_report(session, str(path))
        return path

    def stats(self) -> dict:
        conn = sqlite3.connect(str(DB_PATH))
        total_sessions  = conn.execute("SELECT COUNT(*) FROM sessions").fetchone()[0]
        total_findings  = conn.execute("SELECT COUNT(*) FROM findings").fetchone()[0]
        critical        = conn.execute("SELECT COUNT(*) FROM findings WHERE severity='CRITICAL'").fetchone()[0]
        high            = conn.execute("SELECT COUNT(*) FROM findings WHERE severity='HIGH'").fetchone()[0]
        conn.close()
        return {
            "total_sessions": total_sessions,
            "total_findings": total_findings,
            "critical":       critical,
            "high":           high,
        }
