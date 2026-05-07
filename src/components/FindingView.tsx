import type { Finding } from "../lib/store";

interface Props {
  finding: Finding;
}

export default function FindingView({ finding: f }: Props) {
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
