import { useState } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import "./HomeView.css";

const MODES = [
  { id: "single",  label: "Single",  desc: "One pass with selected tools" },
  { id: "deep",    label: "Deep",    desc: "Exhaustive multi-wave scan"    },
  { id: "passive", label: "Passive", desc: "No active requests sent"       },
];

export default function HomeView() {
  const { tools, appInfo, dbStats, startScanState } = useStore();
  const [target,         setTarget]         = useState("");
  const [mode,           setMode]           = useState("single");
  const [selectedTools,  setSelectedTools]  = useState<string[]>([]);
  const [reportTemplate, setReportTemplate] = useState("");
  const [reportPath,     setReportPath]     = useState("");
  const [launching,      setLaunching]      = useState(false);
  const [error,          setError]          = useState("");

  const installedTools = tools.filter((t) => t.installed && t.enabled);

  function toggleTool(name: string) {
    setSelectedTools((prev) =>
      prev.includes(name) ? prev.filter((n) => n !== name) : [...prev, name]
    );
  }

  async function launch() {
    if (!target.trim()) { setError("Target is required."); return; }
    setError("");
    setLaunching(true);
    try {
      const sessionId = await api.startScan({
        target: target.trim(),
        mode,
        tools: selectedTools,
        report_template:    reportTemplate || null,
        report_output_path: reportPath    || null,
      });
      startScanState({ sessionId, target: target.trim(), mode, selectedTools, reportTemplate, reportPath });
    } catch (e) {
      setError(String(e));
    } finally {
      setLaunching(false);
    }
  }

  return (
    <div className="home-view">
      <div className="home-hero">
        <h1 className="home-title mono">torot<span className="home-v">&nbsp;v4</span></h1>
        <p className="home-sub">Autonomous bug bounty orchestration platform</p>
        {appInfo && (
          <div className="home-tags">
            <span className="home-tag">CLI + Desktop</span>
            <span className="home-tag">{appInfo.knowledge_topics.length} knowledge topics</span>
            {dbStats && <span className="home-tag">{dbStats.sessions} sessions</span>}
          </div>
        )}
      </div>

      <div className="home-form">
        {/* Target */}
        <div className="form-group">
          <label>Target</label>
          <input
            type="text"
            placeholder="https://example.com  •  192.168.1.1  •  /path/to/code"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && launch()}
            autoFocus
          />
        </div>

        {/* Mode */}
        <div className="form-group">
          <label>Mode</label>
          <div className="mode-pills">
            {MODES.map((m) => (
              <button
                key={m.id}
                className={`mode-pill ${mode === m.id ? "active" : ""}`}
                onClick={() => setMode(m.id)}
              >
                <span className="mode-label">{m.label}</span>
                <span className="mode-desc">{m.desc}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Tools */}
        <div className="form-group">
          <label>
            Tools
            {installedTools.length === 0
              ? " — no tools installed, auto-select will be used"
              : ` — ${selectedTools.length === 0 ? "auto-select" : `${selectedTools.length} selected`}`}
          </label>
          {installedTools.length > 0 && (
            <div className="tool-chips">
              {installedTools.map((t) => (
                <button
                  key={t.name}
                  className={`tool-chip ${selectedTools.includes(t.name) ? "selected" : ""}`}
                  onClick={() => toggleTool(t.name)}
                  title={t.description}
                >
                  {t.name}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Report */}
        <details className="adv-details">
          <summary className="adv-summary">Advanced options</summary>
          <div className="adv-body">
            <div className="form-group">
              <label>Report output path (optional)</label>
              <input
                type="text"
                placeholder="/tmp/report.md"
                value={reportPath}
                onChange={(e) => setReportPath(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>Custom report template (optional)</label>
              <textarea
                rows={5}
                placeholder="# My Report&#10;Session: {{session_id}}&#10;..."
                value={reportTemplate}
                onChange={(e) => setReportTemplate(e.target.value)}
                style={{ resize: "vertical" }}
              />
            </div>
          </div>
        </details>

        {error && <div className="form-error">{error}</div>}

        <button
          className="btn btn-primary launch-btn"
          onClick={launch}
          disabled={launching}
        >
          {launching ? "Launching…" : "Launch Scan"}
        </button>
      </div>
    </div>
  );
}
