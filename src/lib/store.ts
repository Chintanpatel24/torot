import { create } from "zustand";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type Severity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFO";
export type Domain   = "blockchain" | "webapp" | "api" | "binary" | "general";
export type Mode     = "single" | "loop" | "daemon";
export type View     = "home" | "scan" | "findings" | "history" | "tools" | "settings";

export interface Finding {
  id:             string;
  session_id:     string;
  tool:           string;
  title:          string;
  severity:       Severity;
  domain:         Domain;
  description:    string;
  file:           string;
  line:           number;
  code_snippet:   string;
  fix_suggestion: string;
  impact:         string;
  bug_type:       string;
  timestamp:      number;
}

export interface StreamLine {
  session_id: string;
  tool:       string;
  line:       string;
  kind:       "output" | "finding" | "system" | "agent";
  severity?:  Severity;
}

export interface ToolStatus {
  name:      string;
  installed: boolean;
  binary:    string;
  domain:    Domain;
  version:   string;
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

export interface ScanState {
  sessionId:     string | null;
  target:        string;
  mode:          Mode;
  running:       boolean;
  streamLines:   StreamLine[];
  findings:      Finding[];
  selectedTools: string[];
}

export interface AppStore {
  // Navigation
  view:          View;
  setView:       (v: View) => void;

  // Tools
  tools:         ToolStatus[];
  setTools:      (t: ToolStatus[]) => void;

  // Scan
  scan:          ScanState;
  setScanTarget: (t: string) => void;
  setScanMode:   (m: Mode) => void;
  toggleTool:    (name: string) => void;
  selectAllTools:(domain?: Domain) => void;
  clearTools:    () => void;
  startScanState:(sessionId: string) => void;
  stopScanState: () => void;
  addStreamLine: (line: StreamLine) => void;
  addFinding:    (f: Finding) => void;
  clearStream:   () => void;

  // History
  sessions:      DbSession[];
  setSessions:   (s: DbSession[]) => void;

  // Active session findings view
  activeFinding: Finding | null;
  setActiveFinding: (f: Finding | null) => void;

  // Stats
  dbStats: { sessions: number; findings: number; critical: number; high: number };
  setDbStats: (s: AppStore["dbStats"]) => void;

  // Loop mode
  loopCount:     number;
  incLoopCount:  () => void;
  resetLoop:     () => void;
}

export const useStore = create<AppStore>((set, get) => ({
  view:    "home",
  setView: (v) => set({ view: v }),

  tools:    [],
  setTools: (t) => set({ tools: t }),

  scan: {
    sessionId:     null,
    target:        "",
    mode:          "single",
    running:       false,
    streamLines:   [],
    findings:      [],
    selectedTools: [],
  },

  setScanTarget: (t) => set((s) => ({ scan: { ...s.scan, target: t } })),
  setScanMode:   (m) => set((s) => ({ scan: { ...s.scan, mode: m } })),

  toggleTool: (name) => set((s) => {
    const sel = s.scan.selectedTools;
    const next = sel.includes(name)
      ? sel.filter((n) => n !== name)
      : [...sel, name];
    return { scan: { ...s.scan, selectedTools: next } };
  }),

  selectAllTools: (domain) => set((s) => {
    const available = s.tools.filter((t) => t.installed && (!domain || t.domain === domain));
    return { scan: { ...s.scan, selectedTools: available.map((t) => t.name) } };
  }),

  clearTools: () => set((s) => ({ scan: { ...s.scan, selectedTools: [] } })),

  startScanState: (sessionId) => set((s) => ({
    scan: { ...s.scan, sessionId, running: true, streamLines: [], findings: [] },
    view: "scan",
  })),

  stopScanState: () => set((s) => ({ scan: { ...s.scan, running: false } })),

  addStreamLine: (line) => set((s) => ({
    scan: {
      ...s.scan,
      streamLines: [...s.scan.streamLines.slice(-800), line],
    },
  })),

  addFinding: (f) => set((s) => ({
    scan: { ...s.scan, findings: [...s.scan.findings, f] },
  })),

  clearStream: () => set((s) => ({ scan: { ...s.scan, streamLines: [] } })),

  sessions:    [],
  setSessions: (s) => set({ sessions: s }),

  activeFinding:    null,
  setActiveFinding: (f) => set({ activeFinding: f }),

  dbStats: { sessions: 0, findings: 0, critical: 0, high: 0 },
  setDbStats: (s) => set({ dbStats: s }),

  loopCount:    0,
  incLoopCount: () => set((s) => ({ loopCount: s.loopCount + 1 })),
  resetLoop:    () => set({ loopCount: 0 }),
}));
