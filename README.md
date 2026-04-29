# 🔍 Torot — Blockchain & Smart Contract Bug Hunter

> **An open-source, agent-style CLI security tool that orchestrates 10 industry-standard analyzers and delivers a unified bug report with a live terminal dashboard.**

```
 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║   
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   
```

---

## ✨ Features

-  **Agent-style orchestration** — Runs all installed tools automatically, in parallel
-  **Live TUI Dashboard** — Real-time terminal dashboard (like `htop`) showing each tool's progress, bug count, and log feed
-  **10 Tools Integrated** — Slither, Aderyn, Mythril, Manticore, Echidna, Securify2, solhint, Oyente, SmartCheck, Halmos
-  **Rich Markdown Report** — Detailed `.md` with bug descriptions, severity, code snippets, and fix suggestions
-  **Concurrent Execution** — Configurable parallelism for fast scans
-  **Rust + Python** — Python orchestration with Rust tooling (Aderyn)
-  **Auto-detection** — Detects Solidity/Rust/EVM files automatically

---

##  Quick Start

### Install Torot

```bash
pip install torot
```

Or from source:

```bash
git clone https://github.com/Chintanpatel24/torot
cd torot
pip install -e .
```

### Run a Scan

```bash
# Scan a smart contracts folder — opens live TUI dashboard
torot ./my-contracts/

# Save report to a custom path
torot ./my-contracts/ --report audit_report.md

# Plain output without dashboard
torot ./my-contracts/ --no-dashboard

# Control concurrency (default: 4 tools in parallel)
torot ./my-contracts/ --concurrent 3
```

---

##  Dashboard

When you run `torot`, a live TUI dashboard opens in your terminal showing:

```
┌─────────────────────────────────────────────────────────────────────┐
│                          TOROT DASHBOARD                             │
│  Target: ./my-contracts/  |  Elapsed: 42.3s                         │
├──────────────────────────────┬───────────────────────────── ─────────┤
│  ⚙ Tool Pipeline             │  🐛 Bug Summary                      │
│                              │                                       │
│  slither    ✔ completed  12  │  💀 CRITICAL   ██                 2  │
│  aderyn     ◉ running     —  │  🔴 HIGH       ████████           8  │
│  mythril    ✔ completed   3  │  🟡 MEDIUM     ██████████████    14  │
│  manticore  ✗ not found   —  │  🔵 LOW        ████████           8  │
│  echidna    ◉ running     —  │  ⚪ INFO       ██                 2  │
│  securify   ○ pending     —  │                                       │
│  solhint    ✔ completed   5  │  📋 Live Log                         │
│  oyente     ✗ not found   —  │  12:34:01 ✔ slither → 12 issues     │
│  smartcheck ✔ completed   1  │  12:34:15 ✔ mythril → 3 issues      │
│  halmos     ✔ completed   3  │  12:34:20 ◉ aderyn → Running...     │
└──────────────────────────────┴──────────────────────────────────────┘
```

---

##  Supported Tools

| Tool | Type | Detects |
|------|------|---------|
| [Slither](https://github.com/crytic/slither) | Static Analysis | Reentrancy, overflow, access control |
| [Aderyn](https://github.com/Cyfrin/aderyn) | Static Analysis | Multi-contract, custom detectors |
| [Mythril](https://github.com/ConsenSys/mythril) | Symbolic Execution | Reentrancy, tx.origin bugs |
| [Manticore](https://github.com/trailofbits/manticore) | Symbolic Execution | Custom security properties |
| [Echidna](https://github.com/crytic/echidna) | Fuzzing | Invariant violations |
| [Securify2](https://github.com/eth-sri/securify2) | Static Analysis | Security pattern compliance |
| [solhint](https://github.com/protofire/solhint) | Linting | Coding standards |
| [Oyente](https://github.com/enzymefinance/oyente) | Static Analysis | Timestamp dependence, reentrancy |
| [SmartCheck](https://github.com/smartdec/smartcheck) | Static Analysis | Known vulnerability patterns |
| [Halmos](https://github.com/a16z/halmos) | Model Checking | SMT-based correctness proofs |

> **Torot skips tools that are not installed** — you don't need all 10. Install the ones you want.

---

##  Installing the Security Tools

### Slither
```bash
pip install slither-analyzer
```

### Aderyn
```bash
cargo install aderyn
```

### Mythril
```bash
pip install mythril
```

### Echidna
```bash
# macOS
brew install echidna

# Linux — download from releases
wget https://github.com/crytic/echidna/releases/latest/download/echidna-linux.zip
unzip echidna-linux.zip && sudo mv echidna /usr/local/bin/
```

### solhint
```bash
npm install -g solhint
```

### Halmos
```bash
pip install halmos
```

### Manticore
```bash
pip install manticore[native]
```

---

## 📋 Sample Report

Torot generates a `torot_report_<timestamp>.md` with:

-  **Executive Summary** — bug counts by severity
-  **Tool Results Table** — which tools ran, how long, how many issues
-  **Detailed Findings** — for each bug:
  - Title, severity, tool, location (file:line)
  - Description of the bug
  - Buggy code snippet
  - Potential impact
  - Fix / recommendation
  - Reference links
-  **Missing Tools** — list of tools not installed

---

##  CLI Reference

```
torot <path> [options]

Arguments:
  path                  Path to the smart contracts / code folder

Options:
  --report FILE, -r     Output path for Markdown report
  --no-dashboard        Disable TUI; use plain terminal output
  --concurrent N, -c    Max tools to run in parallel (default: 4)
  --version, -v         Show version
  --help, -h            Show help
```

---

## Architecture

```
torot/
├── cli.py                  ← CLI entry point (argparse)
├── core/
│   ├── engine.py           ← Scan orchestration engine (asyncio)
│   ├── detector.py         ← File/language detection
│   └── models.py           ← Data models (Bug, ScanSession, ToolResult)
├── scanners/
│   ├── base.py             ← BaseScanner (abstract)
│   ├── slither_scanner.py  ← Slither integration
│   └── all_scanners.py     ← All other tool integrations
├── tui/
│   └── dashboard.py        ← Rich Live TUI dashboard
└── report/
    └── generator.py        ← Markdown report generator
```
