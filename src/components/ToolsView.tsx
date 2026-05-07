import { useState } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import type { ToolProfile } from "../lib/store";

export default function ToolsView() {
  const { tools, setTools } = useStore();
  const [search,  setSearch]  = useState("");
  const [saving,  setSaving]  = useState<string | null>(null);

  const filtered = tools.filter((t) =>
    t.name.toLowerCase().includes(search.toLowerCase()) ||
    t.description.toLowerCase().includes(search.toLowerCase())
  );

  async function toggleEnabled(name: string, enabled: boolean) {
    const tool = tools.find((t) => t.name === name);
    if (!tool) return;
    setSaving(name);
    try {
      const updated = await api.saveToolProfile({ ...tool, enabled } as Partial<ToolProfile>);
      setTools(updated);
    } finally {
      setSaving(null);
    }
  }

  const installed = tools.filter((t) => t.installed).length;

  return (
    <div style={{
      padding: "24px 28px",
      overflow: "auto",
      height: "100%",
      fontFamily: "var(--font-ui)",
    }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 18 }}>
        <div>
          <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)" }}>
            Tool Registry
          </div>
          <div style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            color: "var(--text-muted)",
            marginTop: 2,
          }}>
            {installed}/{tools.length} installed
          </div>
        </div>
        <div style={{ marginLeft: "auto" }}>
          <input
            type="search"
            placeholder="filter tools…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            style={{ width: 180, fontFamily: "var(--font-mono)", fontSize: 11 }}
          />
        </div>
      </div>

      {/* Tool list */}
      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        {filtered.map((tool) => (
          <div
            key={tool.name}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              padding: "9px 12px",
              background: "var(--bg-surface)",
              border: "1px solid var(--border-faint)",
              borderRadius: "var(--radius-md)",
              transition: "border-color 0.1s",
            }}
          >
            {/* Status dot */}
            <span style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: tool.installed ? "var(--green)" : "var(--text-muted)",
              flexShrink: 0,
              opacity: tool.installed ? 1 : 0.4,
            }} />

            {/* Name */}
            <div style={{ width: 90, flexShrink: 0 }}>
              <span style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 500,
                color: "var(--text-primary)",
              }}>
                {tool.name}
              </span>
              {tool.source === "custom" && (
                <span style={{
                  marginLeft: 5,
                  fontSize: 9,
                  color: "var(--blue)",
                  fontFamily: "var(--font-mono)",
                }}>
                  custom
                </span>
              )}
            </div>

            {/* Domain tag */}
            <span style={{
              fontSize: 9,
              fontFamily: "var(--font-mono)",
              color: "var(--text-muted)",
              background: "var(--bg-elevated)",
              border: "1px solid var(--border-subtle)",
              borderRadius: 2,
              padding: "1px 5px",
              flexShrink: 0,
              letterSpacing: "0.06em",
              width: 58,
              textAlign: "center" as const,
            }}>
              {tool.domain}
            </span>

            {/* Description */}
            <span style={{
              flex: 1,
              fontSize: 11,
              color: "var(--text-muted)",
              whiteSpace: "nowrap" as const,
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}>
              {tool.installed && tool.version
                ? tool.version
                : tool.installed
                ? "installed"
                : tool.install_hint}
            </span>

            {/* Toggle */}
            <button
              style={{
                padding: "2px 9px",
                borderRadius: 2,
                border: tool.enabled
                  ? "1px solid var(--green-border)"
                  : "1px solid var(--border-subtle)",
                background: tool.enabled ? "var(--green-dim)" : "transparent",
                color: tool.enabled ? "var(--green)" : "var(--text-muted)",
                fontFamily: "var(--font-mono)",
                fontSize: 9,
                cursor: "pointer",
                opacity: saving === tool.name ? 0.5 : 1,
                letterSpacing: "0.06em",
                flexShrink: 0,
              }}
              onClick={() => toggleEnabled(tool.name, !tool.enabled)}
              disabled={saving === tool.name}
            >
              {tool.enabled ? "enabled" : "disabled"}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
