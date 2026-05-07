import { useState } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import "./HomeView.css";

const MODES = [
  { id: "single",  name: "Single",  desc: "One pass, all tools" },
  { id: "deep",    name: "Deep",    desc: "Multi-wave exhaustive" },
  { id: "passive", name: "Passive", desc: "No active requests"   },
];

export default function HomeView() {
  const { tools, appInfo, dbStats, startScanState } = useStore();
  const [target,        setTarget]        = useState("");
  const [mode,          setMode]          = useState("single");
  const [selected,      setSelected]      = useState<string[]>([]);
  const [reportTemplate,setReportTemplate]= useState("");
  const [reportPath,    setReportPath]    = useState("");
  const [showAdv,       setShowAdv]       = useState(false);
  const [launching,     setLaunching]     = useState(false);
  const [error,         setError]         = useState("");

  const installedTools = tools.filter((t) => t.installed && t.enabled);

  function toggleTool(name: string) {
    setSelected((prev) =>
      prev.includes(name) ? prev.filter((n) => n !== name) : [...prev, name]
    );
  }

  async function launch() {
    if (!target.trim()) { setError("Target required."); return; }
    setError("");
    setLaunching(true);
    try {
      const sessionId = await api.startScan({
        target: target.trim(),
        mode,
        tools: selected,
        report_template:    reportTemplate || null,
        report_output_path: reportPath    || null,
      });
      startScanState({
        sessionId,
        target: target.trim(),
        mode,
        selectedTools: selected,
        reportTemplate,
        reportPath,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setLaunching(false);
    }
  }

  return (
    <div className="home-view">
      <div className="home-header">
        <div className="home-wordmark">
          <span className="home-wordmark-name">torot</span>
          <span className="home-wordmark-ver">v4.0.0</span>
        </div>
        <p className="home-tagline">autonomous bug bounty orchestration</p>
        {(appInfo || dbStats) && (
          <div className="home-meta">
            {appInfo && (
              <span className="home-meta-item">{appInfo.knowledge_topics.length} knowledge topics</span>
            )}
            {dbStats && <span className="home-meta-item">{dbStats.sessions} sessions</span>}
            {dbStats && <span className="home-meta-item">{dbStats.findings} findings</span>}
            <span className="home-meta-item">CLI + desktop</span>
          </div>
        )}
      </div>

      <div className="home-form">
        {/* Target */}
        <div className="form-field">
          <label>Target</label>
          <div className="target-wrap">
            <span className="target-prompt">$</span>
            <input
              className="target-input"
              type="text"
              placeholder="https://example.com  ·  192.168.1.1  ·  /path/to/code"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && launch()}
              autoFocus
            />
          </div>
        </div>

        {/* Mode */}
        <div className="form-field">
          <label>Mode</label>
          <div className="mode-row">
            {MODES.map((m) => (
              <button
                key={m.id}
                className={`mode-pill ${mode === m.id ? "active" : ""}`}
                onClick={() => setMode(m.id)}
              >
                <span className="mode-pill-name">{m.name}</span>
                <span className="mode-pill-desc">{m.desc}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Tools */}
        <div className="form-field">
          <label>
            Tools{" "}
            {selected.length > 0
              ? `— ${selected.length} selected`
              : installedTools.length > 0
              ? "— auto-select"
              : "— none installed"}
          </label>
          {installedTools.length > 0 ? (
            <div className="tool-chips">
              {installedTools.map((t) => (
                <button
                  key={t.name}
                  className={`tool-chip ${selected.includes(t.name) ? "selected" : ""}`}
                  onClick={() => toggleTool(t.name)}
                  title={t.description}
                >
                  {t.name}
                </button>
              ))}
            </div>
          ) : (
            <span className="tool-hint">
              No tools detected. Install nmap, nuclei, semgrep, etc. and restart.
            </span>
          )}
        </div>

        {/* Advanced */}
        <button className="adv-toggle" onClick={() => setShowAdv(!showAdv)}>
          <span>{showAdv ? "▾" : "▸"}</span>
          Advanced options
        </button>
        {showAdv && (
          <div className="adv-body">
            <div className="form-field">
              <label>Report output path</label>
              <input
                type="text"
                placeholder="/tmp/report.md"
                value={reportPath}
                onChange={(e) => setReportPath(e.target.value)}
              />
            </div>
            <div className="form-field">
              <label>Custom report template</label>
              <textarea
                rows={5}
                placeholder={"# Report\nSession: {{session_id}}\n..."}
                value={reportTemplate}
                onChange={(e) => setReportTemplate(e.target.value)}
                style={{ resize: "vertical", fontFamily: "var(--font-mono)", fontSize: 11 }}
              />
            </div>
          </div>
        )}

        {error && <div className="form-error">{error}</div>}

        <div className="launch-row">
          <button
            className="btn btn-primary launch-btn"
            onClick={launch}
            disabled={launching}
          >
            {launching ? "launching…" : "run scan"}
          </button>
        </div>
      </div>
    </div>
  );
}
