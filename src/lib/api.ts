import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppInfo,
  AppConfig,
  DbSession,
  DbStats,
  Finding,
  StreamLine,
  ToolStatus,
  ToolProfile,
} from "./store";

export interface ScanRequest {
  target:              string;
  mode:                string;
  tools:               string[];
  report_template:     string | null;
  report_output_path:  string | null;
}

export interface ReportRequest {
  session_id:  string;
  template:    string | null;
  output_path: string | null;
}

export interface ReportResult {
  session_id: string;
  path:       string;
  summary:    string;
}

export interface ScanCompletePayload {
  session_id:  string;
  total:       number;
  report_path: string | null;
}

export const api = {
  getAppInfo:     () => invoke<AppInfo>("get_app_info"),
  getSettings:    () => invoke<AppConfig>("get_settings"),
  saveSettings:   (config: AppConfig) => invoke<AppConfig>("save_settings", { config }),
  getTools:       () => invoke<ToolStatus[]>("get_tools"),
  saveToolProfile:(profile: Partial<ToolProfile>) => invoke<ToolStatus[]>("save_tool_profile", { profile }),
  startScan:      (request: ScanRequest) => invoke<string>("start_scan", { request }),
  stopScan:       () => invoke<void>("stop_scan"),
  getSessions:    () => invoke<DbSession[]>("get_sessions"),
  getFindings:    (session_id: string) => invoke<Finding[]>("get_findings", { session_id }),
  getDbStats:     () => invoke<DbStats>("get_db_stats"),
  generateReport: (request: ReportRequest) => invoke<ReportResult>("generate_report", { request }),
};

export async function onStreamLine(cb: (line: StreamLine) => void) {
  const unlisten = await listen<StreamLine>("stream_line", (e) => cb(e.payload));
  return unlisten;
}

export async function onNewFinding(cb: (f: Finding) => void) {
  const unlisten = await listen<Finding>("new_finding", (e) => cb(e.payload));
  return unlisten;
}

export async function onScanComplete(cb: (payload: ScanCompletePayload) => void) {
  const unlisten = await listen<ScanCompletePayload>("scan_complete", (e) => cb(e.payload));
  return unlisten;
}
