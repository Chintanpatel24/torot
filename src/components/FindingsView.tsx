import { useStore } from "../lib/store";
import type { Finding, Severity } from "../lib/store";
import "./FindingsView.css";

const SEV_ORDER: Severity[] = ["CRITICAL","HIGH","MEDIUM","LOW","INFO"];

export default function FindingsView() {
  const { scan, activeFinding, setActiveFinding } = useStore();

  const byGroup = SEV_ORDER.reduce<Record<string, Finding[]>>((acc, s) => {
    acc[s] = scan.findings.filter((f) => f.severity === s);
    return acc;
  }, {});

  if (scan.findings.length === 0) {
    return (
      <div className="findings-empty">
        <span className="mono">No findings in current session.</span>
        <p>Start a scan from the Home view to populate findings.</p>
      </div>
    );
  }

  return (
    <div className="findings-view">
      {/* Left: finding list */}
      <div className="findings-list">
        <div className="findings-list-header">
          <span className="section-label">Findings — {scan.findings.length} total</span>
        </div>
        {SEV_ORDER.map((sev) => {
          const items = byGroup[sev] || [];
          if (!items.length) return null;
          return (
            <div key={sev} className="finding-group">
              <div className="group-header">
                <span className={`badge badge-${sev.toLowerCase()}`}>{sev}</span>
                <span className="group-count">{items.length}</span>
              </div>
              {items.map((f) => (
                <button
                  key={f.id}
                  className={`finding-item ${activeFinding?.id === f.id ? "active" : ""}`}
                  onClick={() => setActiveFinding(f)}
                >
                  <span className="fi-tool mono">{f.tool}</span>
                  <span className="fi-title">{f.title.replace(/^\[.*?\]\s*/, "")}</span>
                  {f.file && <span className="fi-loc mono">{f.file.split("/").pop()}{f.line ? `:${f.line}` : ""}</span>}
                </button>
              ))}
            </div>
          );
        })}
      </div>

      {/* Right: detail panel */}
      <div className="finding-detail">
        {!activeFinding ? (
          <div className="detail-empty mono">Select a finding to view details.</div>
        ) : (
          <FindingDetail f={activeFinding} />
        )}
      </div>
    </div>
  );
}

function FindingDetail({ f }: { f: Finding }) {
  return (
    <div className="detail-content">
      <div className="detail-header">
        <span className={`badge badge-${f.severity.toLowerCase()}`}>{f.severity}</span>
        <span className="detail-tool mono">{f.tool}</span>
        {f.domain && <span className="detail-domain">{f.domain}</span>}
      </div>

      <h2 className="detail-title">{f.title.replace(/^\[.*?\]\s*/, "")}</h2>

      {f.file && (
        <div className="detail-location mono">
          {f.file}{f.line ? `:${f.line}` : ""}
        </div>
      )}

      {f.description && (
        <section className="detail-section">
          <span className="section-label">Description</span>
          <p className="detail-body">{f.description}</p>
        </section>
      )}

      {f.code_snippet && (
        <section className="detail-section">
          <span className="section-label">Code</span>
          <pre className="code-block mono">{f.code_snippet}</pre>
        </section>
      )}

      {f.impact && (
        <section className="detail-section">
          <span className="section-label">Impact</span>
          <p className="detail-body impact">{f.impact}</p>
        </section>
      )}

      {f.fix_suggestion && (
        <section className="detail-section">
          <span className="section-label">Recommended Fix</span>
          <pre className="fix-block mono">{f.fix_suggestion}</pre>
        </section>
      )}

      <section className="detail-section">
        <span className="section-label">Finding ID</span>
        <span className="detail-id mono">{f.id}</span>
      </section>
    </div>
  );
}
