// ─── HistoryView ─────────────────────────────────────────────────────────────
import { useEffect } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";

export default function HistoryView() {
  const { sessions, setSessions } = useStore();

  useEffect(() => {
    api.getSessions().then(setSessions).catch(() => {});
  }, []);

  function fmtTime(ts: number) {
    if (!ts) return "—";
    return new Date(ts * 1000).toLocaleString();
  }

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <h2 style={{ color: "var(--text-primary)", marginBottom: 16, fontSize: 16, fontWeight: 600 }}>
        Session History
      </h2>
      {sessions.length === 0 ? (
        <p style={{ color: "var(--text-muted)", fontFamily: "var(--font-mono)", fontSize: 13 }}>
          No past sessions found.
        </p>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, fontFamily: "var(--font-mono)" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-default)", color: "var(--text-muted)" }}>
              {["ID","Target","Domain","Started","Findings"].map((h) => (
                <th key={h} style={{ textAlign: "left", padding: "6px 12px", fontWeight: 600, letterSpacing: "0.05em" }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => (
              <tr key={s.id} style={{ borderBottom: "1px solid var(--border-subtle)" }}>
                <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{s.id}</td>
                <td style={{ padding: "8px 12px", color: "var(--text-primary)", maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.target}</td>
                <td style={{ padding: "8px 12px", color: "var(--text-secondary)" }}>{s.domain}</td>
                <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{fmtTime(s.start_time)}</td>
                <td style={{ padding: "8px 12px", color: s.total_findings > 0 ? "var(--orange)" : "var(--text-muted)", fontWeight: 700 }}>{s.total_findings}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
