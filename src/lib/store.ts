import { create } from "zustand";

export type Severity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFO";
export type View = "home" | "scan" | "findings" | "history" | "tools" | "settings";

export interface Finding {
  id:            string;
  session_id:    string;
  tool:          string;
  title:         string;
  severity:      Severity;
  domain:        string;
  description:   string;
  file:          string;
  line:          number;
  code_snippet:  string;
  fix_suggestion:string;
  impact:        string;
  bug_type:      string;
  timestamp:     number;
}

export interface StreamLine {
  session_id: string;
  tool:       string;
  line:       string;
  kind:       string;
  severity?:  string;
}

export interface DbSession {
  id:             string;
  target:         string;
  domain:         string;
  start_time:     number;
  end_time:       number;
  total_findings: number;
  summary:        string;
}

export interface ToolStatus {
  name:         string;
  installed:    boolean;
  binary:       string;
  version:      string;
  domain:       string;
  description:  string;
  install_hint: string;
  output_format:string;
  source:       string;
  auto_detect:  boolean;
  enabled:      boolean;
  capabilities: string[];
  knowledge:    string[];
  wizard_steps: WizardStep[];
}

export interface WizardStep {
  order:  number;
  title:  string;
  detail: string;
}

export interface SandboxConfig {
  profile:              string;
  max_runtime_seconds:  number;
  allow_network:        boolean;
  writable_reports_only:boolean;
}

export interface ToolProfile {
  name:            string;
  domain:          string;
  description:     string;
  binary_names:    string[];
  path_override:   string | null;
  args:            string[];
  version_args:    string[];
  install_hint:    string;
  output_format:   string;
  input_kinds:     string[];
  source:          string;
  auto_detect:     boolean;
  enabled:         boolean;
  timeout_seconds: number;
  capabilities:    string[];
  knowledge:       string[];
}

export interface AppConfig {
  version:                  string;
  install_mode:             string;
  default_report_template:  string;
  sandbox:                  SandboxConfig;
  tools:                    ToolProfile[];
  knowledge_topics:         string[];
}

export interface AppInfo {
  version:                      string;
  install_mode:                 string;
  cli_supported:                boolean;
  knowledge_topics:             string[];
  report_template_placeholders: string[];
}

export interface DbStats {
  sessions: number;
  findings: number;
  critical: number;
  high:     number;
}

interface ScanState {
  running:           boolean;
  sessionId:         string | null;
  target:            string;
  mode:              string;
  selectedTools:     string[];
  streamLines:       StreamLine[];
  findings:          Finding[];
  reportTemplate:    string;
  reportPath:        string;
  generatedReportPath: string | null;
}

interface AppStore {
  view:           View;
  scan:           ScanState;
  sessions:       DbSession[];
  tools:          ToolStatus[];
  config:         AppConfig | null;
  appInfo:        AppInfo | null;
  dbStats:        DbStats | null;
  activeFinding:  Finding | null;

  setView:        (v: View) => void;
  setTools:       (tools: ToolStatus[]) => void;
  setConfig:      (config: AppConfig) => void;
  setAppInfo:     (info: AppInfo) => void;
  setDbStats:     (stats: DbStats) => void;
  setSessions:    (sessions: DbSession[]) => void;
  setActiveFinding:(f: Finding | null) => void;

  startScanState: (params: {
    sessionId:     string;
    target:        string;
    mode:          string;
    selectedTools: string[];
    reportTemplate:string;
    reportPath:    string;
  }) => void;
  stopScanState:        () => void;
  addStreamLine:        (line: StreamLine) => void;
  addFinding:           (f: Finding) => void;
  setGeneratedReportPath:(path: string) => void;
}

export const useStore = create<AppStore>((set) => ({
  view:          "home",
  scan: {
    running:             false,
    sessionId:           null,
    target:              "",
    mode:                "single",
    selectedTools:       [],
    streamLines:         [],
    findings:            [],
    reportTemplate:      "",
    reportPath:          "",
    generatedReportPath: null,
  },
  sessions:      [],
  tools:         [],
  config:        null,
  appInfo:       null,
  dbStats:       null,
  activeFinding: null,

  setView:         (v) => set({ view: v }),
  setTools:        (tools) => set({ tools }),
  setConfig:       (config) => set({ config }),
  setAppInfo:      (appInfo) => set({ appInfo }),
  setDbStats:      (dbStats) => set({ dbStats }),
  setSessions:     (sessions) => set({ sessions }),
  setActiveFinding:(f) => set({ activeFinding: f }),

  startScanState: (params) =>
    set((s) => ({
      view: "scan",
      scan: {
        ...s.scan,
        running:             true,
        sessionId:           params.sessionId,
        target:              params.target,
        mode:                params.mode,
        selectedTools:       params.selectedTools,
        streamLines:         [],
        findings:            [],
        reportTemplate:      params.reportTemplate,
        reportPath:          params.reportPath,
        generatedReportPath: null,
      },
    })),

  stopScanState: () =>
    set((s) => ({ scan: { ...s.scan, running: false } })),

  addStreamLine: (line) =>
    set((s) => ({
      scan: {
        ...s.scan,
        streamLines: [...s.scan.streamLines, line].slice(-2000),
      },
    })),

  addFinding: (f) =>
    set((s) => ({ scan: { ...s.scan, findings: [...s.scan.findings, f] } })),

  setGeneratedReportPath: (path) =>
    set((s) => ({ scan: { ...s.scan, generatedReportPath: path } })),
}));
