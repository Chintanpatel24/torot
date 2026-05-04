import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { ToolStatus, Finding, DbSession, StreamLine } from "./store";

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

export const api = {
  getTools: (): Promise<ToolStatus[]> =>
    invoke("get_tools"),

  startScan: (target: string, mode: string, tools: string[]): Promise<string> =>
    invoke("start_scan", { target, mode, tools }),

  stopScan: (): Promise<void> =>
    invoke("stop_scan"),

  getSessions: (): Promise<DbSession[]> =>
    invoke("get_sessions"),

  getFindings: (sessionId: string): Promise<Finding[]> =>
    invoke("get_findings", { sessionId }),

  getDbStats: (): Promise<{ sessions: number; findings: number; critical: number; high: number }> =>
    invoke("get_db_stats"),
};

// ─────────────────────────────────────────────────────────────────────────────
// Event listeners
// ─────────────────────────────────────────────────────────────────────────────

