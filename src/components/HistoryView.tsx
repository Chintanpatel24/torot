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
    return new Date(ts * 1000).toLocaleString(undefined, {
      month: "short", day: "numeric",
      hour: "2-digit", minute: "2-digit",
    });
  }

  const mono: React.CSSProperties = {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
  };

  return (
    <div style={{ padding: "24px 28px", overflow: "auto", height: "100%" }}>
      <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: 18 }}>
        Session History
      </div>

      {sessions.length === 0 ? (
        <p style={{ ...mono, color: "var(--text-muted)" }}>No past sessions.</p>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              {["ID", "Target", "Started", "Findings", "Summary"].map((h) => (
                <th key={h} style={{
                  textAlign: "left",
                  padding: "0 12px 8px 0",
                  fontSize: 9,
                  fontWeight: 600,
                  letterSpacing: "0.1em",
                  textTransform: "uppercase",
                  color: "var(--text-muted)",
                  borderBottom: "1px solid var(--border-faint)",
                }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => (
              <tr key={s.id} style={{ borderBottom: "1px solid var(--border-faint)" }}>
                <td style={{ ...mono, padding: "9px 12px 9px 0", color: "var(--text-muted)" }}>
                  {s.id}
                </td>
                <td style={{
                  ...mono,
                  padding: "9px 12px 9px 0",
                  color: "var(--text-primary)",
                  maxWidth: 260,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}>
                  {s.target}
                </td>
                <td style={{ ...mono, padding: "9px 12px 9px 0", color: "var(--text-muted)", whiteSpace: "nowrap" }}>
                  {fmtTime(s.start_time)}
                </td>
                <td style={{
                  ...mono,
                  padding: "9px 12px 9px 0",
                  fontWeight: 700,
                  color: s.total_findings > 0 ? "var(--orange)" : "var(--text-muted)",
                }}>
                  {s.total_findings || "—"}
                </td>
                <td style={{
                  ...mono,
                  padding: "9px 0",
                  color: "var(--text-muted)",
                  maxWidth: 300,
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  fontSize: 10,
                }}>
                  {s.summary || "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
