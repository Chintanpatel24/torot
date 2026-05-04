import { useState } from "react";
import { useStore, type Mode, type Domain } from "../lib/store";
import { api } from "../lib/api";
import { open } from "@tauri-apps/plugin-dialog";
import "./HomeView.css";

const MODES: { id: Mode; label: string; desc: string }[] = [
  { id: "single", label: "Single",  desc: "Run once, show results, exit" },
  { id: "loop",   label: "Loop",    desc: "Repeat scan until stopped" },
  { id: "daemon", label: "Daemon",  desc: "Watch folder, scan on change" },
];

const DOMAINS: { id: Domain | "all"; label: string }[] = [
  { id: "all",        label: "All" },
  { id: "blockchain", label: "Blockchain" },
  { id: "webapp",     label: "Web App" },
  { id: "api",        label: "API" },
  { id: "binary",     label: "Binary" },
];

export default function HomeView() {
  const {
    scan, tools, setScanTarget, setScanMode,
    toggleTool, selectAllTools, clearTools,
    startScanState, setView,
  } = useStore();

  const [filterDomain, setFilterDomain] = useState<Domain | "all">("all");
  const [launching,    setLaunching]    = useState(false);
  const [error,        setError]        = useState("");

  const visibleTools = filterDomain === "all"
    ? tools
    : tools.filter((t) => t.domain === filterDomain);

  const installedVisible = visibleTools.filter((t) => t.installed);
  const missingVisible   = visibleTools.filter((t) => !t.installed);

  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === "string") {
      setScanTarget(selected);
    }
  }

  async function launchScan() {
    setError("");
    if (!scan.target.trim()) { setError("Enter a target path, URL, or contract address."); return; }
    if (scan.selectedTools.length === 0) { setError("Select at least one tool."); return; }
    setLaunching(true);
    try {
      const sessionId = await api.startScan(scan.target, scan.mode, scan.selectedTools);
      startScanState(sessionId);
    } catch (e) {
      setError(String(e));
    } finally {
      setLaunching(false);
    }
  }

  const allInstalledSelected =
    installedVisible.length > 0 &&
    installedVisible.every((t) => scan.selectedTools.includes(t.name));

  return (
    <div className="home-view">
      {/* Target input */}
      <section className="home-section">
        <label className="section-label">Target</label>
        <div className="target-row">
          <input
            className="target-input mono"
            placeholder="./contracts/  or  https://target.com  or  0x1234...abcd"
            value={scan.target}
            onChange={(e) => setScanTarget(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && launchScan()}
          />
          <button className="btn-secondary" onClick={pickFolder}>Browse</button>
        </div>
        <p className="input-hint">Accepts: folder path, URL, Ethereum address, or plain question</p>
      </section>

      {/* Mode picker */}
      <section className="home-section">
        <label className="section-label">Run Mode</label>
        <div className="mode-row">
          {MODES.map((m) => (
            <button
              key={m.id}
              className={`mode-card ${scan.mode === m.id ? "active" : ""}`}
              onClick={() => setScanMode(m.id)}
            >
              <span className="mode-label">{m.label}</span>
              <span className="mode-desc">{m.desc}</span>
            </button>
          ))}
        </div>
      </section>

      {/* Tool selector */}
      <section className="home-section tools-section">
        <div className="tools-header">
          <label className="section-label">Tools</label>
          <div className="domain-filter">
            {DOMAINS.map((d) => (
              <button
                key={d.id}
                className={`domain-btn ${filterDomain === d.id ? "active" : ""}`}
                onClick={() => setFilterDomain(d.id)}
              >
                {d.label}
              </button>
            ))}
          </div>
          <div className="tool-actions">
            <button
              className="btn-xs"
              onClick={() => filterDomain === "all"
                ? selectAllTools()
                : selectAllTools(filterDomain as Domain)
              }
            >
              Select all installed
            </button>
            <button className="btn-xs danger" onClick={clearTools}>Clear</button>
          </div>
        </div>

        <div className="tool-grid">
          {installedVisible.map((t) => (
            <button
              key={t.name}
              className={`tool-chip installed ${scan.selectedTools.includes(t.name) ? "selected" : ""}`}
              onClick={() => toggleTool(t.name)}
            >
              <span className="tool-dot installed" />
              <span className="tool-name">{t.name}</span>
              <span className="tool-domain">{t.domain}</span>
            </button>
          ))}
          {missingVisible.map((t) => (
            <button
              key={t.name}
              className="tool-chip missing"
              disabled
              title={`Not installed`}
            >
              <span className="tool-dot missing" />
              <span className="tool-name">{t.name}</span>
              <span className="tool-domain">{t.domain}</span>
            </button>
          ))}
        </div>

        <p className="tools-hint">
          {scan.selectedTools.length} tool(s) selected
          {missingVisible.length > 0 && ` — ${missingVisible.length} not installed`}
        </p>
      </section>

      {/* Launch */}
      {error && <div className="error-banner">{error}</div>}

      <div className="launch-row">
        <button
          className="btn-launch"
          onClick={launchScan}
          disabled={launching}
        >
          {launching ? "Starting..." : "Start Scan"}
        </button>
        <button className="btn-secondary" onClick={() => setView("tools")}>
          Manage Tools
        </button>
      </div>
    </div>
  );
}
