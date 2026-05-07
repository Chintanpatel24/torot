<div align=center>
  
<img width="250" alt="torot" src="images/torot.png" />

![Torot](https://img.shields.io/badge/version-3.1.1-amber?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue?style=flat-square)

</div>

# Torot

## Features

- **Auto-detects** nmap, bbot, nuclei, httpx, subfinder, amass, katana, ffuf, gobuster, nikto, sqlmap, semgrep, trufflehog, gitleaks, and more
- **Parallel execution** with per-tool timeouts and circuit-breaker resilience
- **Structured findings** — severity-ranked (CRITICAL → INFO), deduplicated, stored in SQLite
- **Markdown report generation** with a customisable template
- **Desktop GUI** (Tauri) and **CLI** in the same binary
- **Session history** across launches

## Quick start

```bash
# Clone
git clone https://github.com/Chintanpatel24/torot.git
cd torot

# Install JS dependencies
npm install

# Run in development (opens desktop window)
npm run tauri:dev

# Build for production
npm run tauri:build
```

## One-liner install

### Linux / macOS
```bash
curl -fsSL https://raw.githubusercontent.com/Chintanpatel24/torot/main/install.sh | bash
```

### Windows (PowerShell / WSL)
```powershell
# WSL (recommended)
bash -c "curl -fsSL https://raw.githubusercontent.com/Chintanpatel24/torot/main/install.sh | bash"
```

### From source
```bash
git clone https://github.com/Chintanpatel24/torot
cd torot
chmod +x install.sh && ./install.sh
```

## CLI usage

Once built, the binary also functions as a CLI tool:

```bash
# List detected tools
torot tools

# Run a scan
torot scan --target https://example.com --tools nmap,nuclei --mode single

# Re-generate a report from a past session
torot report --session <session-id> --output report.md

# Print current config as JSON
torot config
```

---

## Tool auto-detection

Torot looks for tools on `PATH` and in any path overrides you set in Settings. Unsupported tools can be added via the Tool Registry in the UI or by editing `~/.torot/config.json`.

---

## For more : [visit usage.md](usage.md)
