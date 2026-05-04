import { useEffect } from "react";
import { useStore, type Domain } from "../lib/store";
import { api } from "../lib/api";

const DOMAIN_COLORS: Record<string, string> = {
  blockchain: "#f07a1a",
  webapp:     "#4a90d9",
  api:        "#4caf7d",
  binary:     "#ab7ae0",
};

const INSTALL_HINTS: Record<string, string> = {
  slither:    "pip install slither-analyzer",
  aderyn:     "cargo install aderyn",
  mythril:    "pip install mythril",
  echidna:    "brew install echidna",
  manticore:  "pip install manticore[native]",
  solhint:    "npm install -g solhint",
  halmos:     "pip install halmos",
  semgrep:    "pip install semgrep",
  solc:       "pip install solc-select && solc-select install latest",
  wake:       "pip install eth-wake",
  heimdall:   "cargo install heimdall-rs",
  "cargo-audit": "cargo install cargo-audit",
  clippy:     "rustup component add clippy",
  nuclei:     "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest",
  nikto:      "apt install nikto",
  sqlmap:     "pip install sqlmap",
  ffuf:       "go install github.com/ffuf/ffuf/v2@latest",
  gobuster:   "go install github.com/OJ/gobuster/v3@latest",
  dalfox:     "go install github.com/hahwul/dalfox/v2@latest",
  trufflehog: "go install github.com/trufflesecurity/trufflehog/v3@latest",
  gitleaks:   "go install github.com/zricethezav/gitleaks/v8@latest",
  arjun:      "pip install arjun",
  jwt_tool:   "pip install jwt_tool",
  radare2:    "brew install radare2",
  binwalk:    "pip install binwalk",
  checksec:   "pip install checksec",
  strings:    "pre-installed (Linux/macOS)",
  objdump:    "pre-installed (binutils)",
};

export default function ToolsView() {
  const { tools, setTools } = useStore();

  useEffect(() => {
    api.getTools().then(setTools).catch(() => {});
  }, []);

  const domains = [...new Set(tools.map((t) => t.domain))] as Domain[];
  const installed = tools.filter((t) => t.installed).length;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%", display: "flex", flexDirection: "column", gap: 20 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <h2 style={{ color: "var(--text-primary)", fontSize: 16, fontWeight: 600 }}>
          Security Tools
        </h2>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--green)", background: "var(--green-dim)", border: "1px solid rgba(76,175,125,0.25)", borderRadius: 3, padding: "2px 8px" }}>
          {installed} / {tools.length} installed
        </span>
      </div>

      {domains.map((domain) => {
        const domainTools = tools.filter((t) => t.domain === domain);
        const color = DOMAIN_COLORS[domain] || "var(--text-muted)";
        return (
          <div key={domain}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
              <div style={{ width: 3, height: 14, background: color, borderRadius: 2 }} />
              <span style={{ fontSize: 12, fontWeight: 700, color, textTransform: "uppercase", letterSpacing: "0.08em" }}>
                {domain}
              </span>
              <span style={{ fontSize: 11, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
                {domainTools.filter((t) => t.installed).length}/{domainTools.length}
              </span>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 6 }}>
              {domainTools.map((t) => (
                <div key={t.name} style={{
                  background: "var(--bg-surface)",
                  border: `1px solid ${t.installed ? "var(--border-default)" : "var(--border-subtle)"}`,
                  borderRadius: "var(--radius-md)",
                  padding: "10px 12px",
                  opacity: t.installed ? 1 : 0.55,
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <div style={{ width: 6, height: 6, borderRadius: "50%", background: t.installed ? "var(--green)" : "var(--text-muted)", flexShrink: 0 }} />
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, fontWeight: 700, color: "var(--text-primary)" }}>
                      {t.name}
                    </span>
                    {t.installed && (
                      <span style={{ fontSize: 9, color: "var(--green)", background: "var(--green-dim)", borderRadius: 2, padding: "1px 5px", marginLeft: "auto", fontFamily: "var(--font-mono)" }}>
                        installed
                      </span>
                    )}
                  </div>
                  {!t.installed && INSTALL_HINTS[t.name] && (
                    <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--text-muted)", background: "var(--bg-elevated)", borderRadius: 3, padding: "4px 7px", marginTop: 2 }}>
                      {INSTALL_HINTS[t.name]}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}
