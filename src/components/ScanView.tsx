import { useRef, useEffect, useState, type KeyboardEvent } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import "./ScanView.css";

const SEV_CLASS: Record<string, string> = {
  CRITICAL: "sev-critical",
  HIGH:     "sev-high",
  MEDIUM:   "sev-medium",
  LOW:      "sev-low",
  INFO:     "sev-info",
};

export default function ScanView() {
  const { scan, stopScanState, setView, setActiveFinding, setGeneratedReportPath } = useStore();
  const streamRef   = useRef<HTMLDivElement>(null);
  const inputRef    = useRef<HTMLInputElement>(null);
  const [input,     setInput]     = useState("");
  const [autoScroll,setAutoScroll]= useState(true);
  const [activeTab, setActiveTab] = useState<"stream"|"findings">("stream");

  // Auto-scroll stream pane
  useEffect(() => {
    if (autoScroll && streamRef.current) {
      streamRef.current.scrollTop = streamRef.current.scrollHeight;
    }
  }, [scan.streamLines, autoScroll]);

  function handleScroll() {
    if (!streamRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = streamRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 40);
  }

  async function stopScan() {
    await api.stopScan();
    stopScanState();
  }

  async function exportReport() {
    if (!scan.sessionId) return;
    const result = await api.generateReport({
      session_id: scan.sessionId,
      template: scan.reportTemplate || null,
      output_path: scan.reportPath || null,
    });
    setGeneratedReportPath(result.path);
  }

  function handleInput(e: KeyboardEvent) {
    if (e.key !== "Enter" || !input.trim()) return;
    setInput("");
    // Inject as a stream line (agent response simulation)
  }

  function openFinding(f: typeof scan.findings[0]) {
    setActiveFinding(f);
    setView("findings");
  }

  const critCount = scan.findings.filter((f) => f.severity === "CRITICAL").length;
  const highCount = scan.findings.filter((f) => f.severity === "HIGH").length;

  return (
    <div className="scan-view">
      {/* ── Scan header bar ───────────────────────────────────────────── */}
      <div className="scan-header">
        <div className="scan-header-left">
          <span className={`scan-status-dot ${scan.running ? "running" : "done"}`} />
          <span className="scan-target mono">{scan.target}</span>
          <span className="scan-mode-tag">{scan.mode.toUpperCase()}</span>
        </div>
        <div className="scan-header-counts">
          {critCount > 0 && <span className="sev-badge critical">{critCount} CRITICAL</span>}
          {highCount > 0 && <span className="sev-badge high">{highCount} HIGH</span>}
          <span className="total-badge">{scan.findings.length} total</span>
        </div>
        <div className="scan-header-right">
          <button
            className={`tab-btn ${activeTab === "stream" ? "active" : ""}`}
            onClick={() => setActiveTab("stream")}
          >Output</button>
          <button
            className={`tab-btn ${activeTab === "findings" ? "active" : ""}`}
            onClick={() => setActiveTab("findings")}
          >Findings</button>
          {scan.running && (
            <button className="btn-stop" onClick={stopScan}>Stop</button>
          )}
          {!scan.running && scan.sessionId && (
            <button className="btn-stop" onClick={exportReport}>Write Report</button>
          )}
        </div>
      </div>

      {/* ── Main split area ───────────────────────────────────────────── */}
      <div className="scan-body">
        {/* Top: stream pane */}
        <div
          className="stream-pane"
          ref={streamRef}
          onScroll={handleScroll}
        >
          {activeTab === "stream" ? (
            scan.streamLines.length === 0 ? (
              <div className="stream-empty mono">Waiting for output...</div>
            ) : (
              scan.streamLines.map((line, i) => (
                <div key={i} className={`stream-line kind-${line.kind} ${line.severity ? SEV_CLASS[line.severity] || "" : ""}`}>
                  <span className="stream-tool mono">{line.tool.padEnd(12)}</span>
                  <span className="stream-text mono">{line.line}</span>
                </div>
              ))
            )
          ) : (
            /* Findings inline list */
            <div className="findings-inline">
              {scan.findings.length === 0 ? (
                <div className="stream-empty mono">No findings yet...</div>
              ) : (
                scan.findings.map((f) => (
                  <div
                    key={f.id}
                    className={`finding-row ${SEV_CLASS[f.severity] || ""}`}
                    onClick={() => openFinding(f)}
                  >
                    <span className={`badge badge-${f.severity.toLowerCase()}`}>{f.severity}</span>
                    <span className="finding-tool mono">{f.tool}</span>
                    <span className="finding-title">{f.title}</span>
                    {f.file && <span className="finding-loc mono">{f.file}{f.line ? `:${f.line}` : ""}</span>}
                  </div>
                ))
              )}
            </div>
          )}
        </div>

        {/* Bottom: swarm task bar */}
        <div className="swarm-bar">
          <div className="swarm-tasks">
            {scan.selectedTools.map((tool) => {
              const toolFindings = scan.findings.filter((f) => f.tool === tool);
              const hasOutput    = scan.streamLines.some((l) => l.tool === tool);
              const isDone       = !scan.running || scan.streamLines.some((l) => l.tool === tool && l.line.includes("Done"));
              return (
                <div key={tool} className={`swarm-task ${hasOutput ? "active" : ""} ${isDone && !scan.running ? "done" : ""}`}>
                  <span className={`swarm-task-dot ${hasOutput ? (isDone ? "done" : "running") : "pending"}`} />
                  <span className="swarm-task-name mono">{tool}</span>
                  {toolFindings.length > 0 && (
                    <span className="swarm-task-count">{toolFindings.length}</span>
                  )}
                </div>
              );
            })}
          </div>
          {scan.generatedReportPath && (
            <div className="stream-empty mono" style={{ marginLeft: "auto" }}>
              report: {scan.generatedReportPath}
            </div>
          )}
        </div>

        {/* Bottom: chat/command input */}
        <div className="chat-input-bar">
          <div className="chat-prompt-prefix mono">$</div>
          <input
            ref={inputRef}
            className="chat-input mono"
            placeholder="Ask a question or type a command..."
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleInput}
          />
          {!autoScroll && (
            <button
              className="scroll-btn"
              onClick={() => { setAutoScroll(true); }}
            >
              Scroll to bottom
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
