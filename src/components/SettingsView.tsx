import { useState, useEffect } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import type { AppConfig } from "../lib/store";

export default function SettingsView() {
  const { config, setConfig } = useStore();
  const [draft,  setDraft]  = useState<AppConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved,  setSaved]  = useState(false);

  useEffect(() => {
    if (config) setDraft(JSON.parse(JSON.stringify(config)));
  }, [config]);

  if (!draft) return (
    <div style={{ padding: 28, color: "var(--text-muted)", fontFamily: "var(--font-mono)", fontSize: 12 }}>
      loading…
    </div>
  );

  async function save() {
    if (!draft) return;
    setSaving(true);
    try {
      const updated = await api.saveSettings(draft);
      setConfig(updated);
      setDraft(JSON.parse(JSON.stringify(updated)));
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  }

  const row = (label: string, children: React.ReactNode) => (
    <div style={{
      display: "flex",
      alignItems: "center",
      gap: 16,
      padding: "9px 14px",
      borderBottom: "1px solid var(--border-faint)",
    }}>
      <span style={{
        width: 160,
        fontSize: 11,
        color: "var(--text-muted)",
        flexShrink: 0,
        fontFamily: "var(--font-ui)",
      }}>
        {label}
      </span>
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  );

  return (
    <div style={{
      padding: "24px 28px",
      overflow: "auto",
      height: "100%",
      maxWidth: 640,
    }}>
      <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: 20 }}>
        Settings
      </div>

      {/* General */}
      <div style={{ marginBottom: 20 }}>
        <div style={{
          fontSize: 9,
          fontWeight: 600,
          letterSpacing: "0.1em",
          textTransform: "uppercase" as const,
          color: "var(--text-muted)",
          marginBottom: 8,
        }}>
          General
        </div>
        <div style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border-faint)",
          borderRadius: "var(--radius-md)",
          overflow: "hidden",
        }}>
          {row("Install mode",
            <select
              value={draft.install_mode}
              onChange={(e) => setDraft({ ...draft, install_mode: e.target.value })}
              style={{ fontSize: 11 }}
            >
              <option value="both">Desktop + CLI</option>
              <option value="desktop">Desktop only</option>
              <option value="cli">CLI only</option>
            </select>
          )}
        </div>
      </div>

      {/* Sandbox */}
      <div style={{ marginBottom: 20 }}>
        <div style={{
          fontSize: 9,
          fontWeight: 600,
          letterSpacing: "0.1em",
          textTransform: "uppercase" as const,
          color: "var(--text-muted)",
          marginBottom: 8,
        }}>
          Sandbox
        </div>
        <div style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border-faint)",
          borderRadius: "var(--radius-md)",
          overflow: "hidden",
        }}>
          {row("Profile",
            <select
              value={draft.sandbox.profile}
              onChange={(e) => setDraft({ ...draft, sandbox: { ...draft.sandbox, profile: e.target.value }})}
              style={{ fontSize: 11 }}
            >
              <option value="strong">Strong</option>
              <option value="moderate">Moderate</option>
              <option value="off">Off</option>
            </select>
          )}
          {row("Max runtime (s)",
            <input
              type="text"
              value={draft.sandbox.max_runtime_seconds}
              onChange={(e) => setDraft({
                ...draft,
                sandbox: { ...draft.sandbox, max_runtime_seconds: Number(e.target.value) || 900 },
              })}
              style={{ fontFamily: "var(--font-mono)", fontSize: 11, width: 80 }}
            />
          )}
        </div>
      </div>

      {/* Report template */}
      <div style={{ marginBottom: 20 }}>
        <div style={{
          fontSize: 9,
          fontWeight: 600,
          letterSpacing: "0.1em",
          textTransform: "uppercase" as const,
          color: "var(--text-muted)",
          marginBottom: 8,
        }}>
          Default report template
        </div>
        <textarea
          rows={10}
          value={draft.default_report_template}
          onChange={(e) => setDraft({ ...draft, default_report_template: e.target.value })}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            resize: "vertical",
            background: "var(--bg-surface)",
            border: "1px solid var(--border-faint)",
            borderRadius: "var(--radius-md)",
            color: "var(--text-secondary)",
            lineHeight: 1.6,
          }}
        />
      </div>

      <button
        className="btn btn-primary"
        onClick={save}
        disabled={saving}
        style={{ fontSize: 12 }}
      >
        {saving ? "saving…" : saved ? "saved ✓" : "save settings"}
      </button>
    </div>
  );
}
