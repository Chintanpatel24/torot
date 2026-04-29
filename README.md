# Torot — Blockchain & Smart Contract Security Scanner

An open-source, agent-style CLI tool that orchestrates 17 industry-standard
security analyzers, then produces a unified report with reproduction guides,
Foundry tests, PoC scripts, video recording instructions, and official
disclosure templates.

```
 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║   
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   
```

---

## Features

- Works with any number of installed tools — even just 1
- Live terminal dashboard (like htop) showing each tool's real-time status
- 17 tools integrated across Solidity and Rust codebases
- Full per-bug reproduction section:
  - Step-by-step exploit steps
  - Python Proof-of-Concept script
  - Foundry test skeleton
  - Video recording guide (OBS, asciinema)
  - Official disclosure template for Immunefi, Code4rena, Sherlock, HackerOne
- API integrations: OpenAI GPT-4, Anthropic Claude, Etherscan, GitHub
- Runs all tools concurrently with configurable parallelism
- Gracefully skips tools that are not installed
- Clean, professional Markdown report output

---

## Quick Start

```bash
# Install
pip install torot

# Or from source
git clone https://github.com/your-org/torot
cd torot
pip install -e .

# Check which tools you have installed
torot --list-tools

# Scan a folder (opens live TUI dashboard)
torot ./my-contracts/

# Save report to a custom path
torot ./my-contracts/ --report audit.md

# Plain output — no dashboard
torot ./my-contracts/ --no-dashboard

# Run with fewer parallel tools
torot ./my-contracts/ --concurrent 2
```

---

## API Integrations

Pass API keys with `--api key=value`. Repeat for multiple keys.

```bash
# OpenAI GPT-4 analysis on each finding
torot ./contracts/ --api openai=sk-...

# Anthropic Claude fix suggestions
torot ./contracts/ --api anthropic=sk-ant-...

# Etherscan contract verification check
torot ./contracts/ --api etherscan=ABCDEF123

# Auto-open GitHub issues for CRITICAL and HIGH findings
torot ./contracts/ --api github=ghp_TOKEN --api github-repo=owner/repo

# Combine multiple APIs
torot ./contracts/ \
  --api anthropic=sk-ant-... \
  --api etherscan=ABCDEF \
  --api github=ghp_TOKEN \
  --api github-repo=myorg/myrepo
```

### What each API does

| Flag | Effect |
|------|--------|
| `openai=<key>` | GPT-4 analyses each CRITICAL/HIGH/MEDIUM bug and rewrites the fix suggestion |
| `anthropic=<key>` | Claude analyses each CRITICAL/HIGH/MEDIUM bug and rewrites the fix suggestion |
| `etherscan=<key>` | Checks whether any detected contract addresses are verified on Etherscan |
| `github=<token>` + `github-repo=owner/repo` | Creates a GitHub issue for each CRITICAL and HIGH finding automatically |

---

## Supported Tools

Run `torot --list-tools` to see which are installed on your machine.

| Tool | Language | What it finds |
|------|----------|---------------|
| Slither | Solidity | Reentrancy, overflow, access control |
| Aderyn | Solidity | Multi-contract issues, custom detectors |
| Mythril | Solidity | Reentrancy, tx.origin (symbolic execution) |
| Manticore | Solidity | Custom security property violations |
| Echidna | Solidity | Invariant and assertion violations (fuzzing) |
| Securify2 | Solidity | Security pattern compliance |
| solhint | Solidity | Coding standards, linting |
| Oyente | Solidity | Timestamp dependence, reentrancy |
| SmartCheck | Solidity | Known vulnerability patterns (XPath) |
| Halmos | Solidity | Formal verification (SMT) |
| Semgrep | Solidity/Rust | Pattern-based custom rules |
| cargo-audit | Rust | Vulnerable dependencies (RustSec DB) |
| Clippy | Rust | Unsafe patterns, common mistakes |
| solc | Solidity | Compiler warnings and errors |
| Pyrometer | Solidity | Arithmetic bounds and range errors |
| Wake | Solidity | Analysis framework with plugins |
| 4naly3er | Solidity | Audit-contest-style report findings |

Torot skips tools not installed in PATH — you do not need all 17.
Install only the tools you want and Torot adapts automatically.

---

## Report Contents

Each bug in the report includes:

1. Severity, tool, type, and location (file:line)
2. Description
3. Vulnerable code snippet
4. Where the bug appears in a production deployment
5. AI analysis (if an API key is provided)
6. Impact
7. Recommended fix (code-level)
8. References (SWC, audit wiki, etc.)
9. Full reproduction guide (inside a collapsible section):
   - Environment setup
   - Step-by-step exploitation walkthrough
   - Python PoC exploit script
   - Foundry test skeleton (`forge test`)
   - Video recording guide (OBS setup, script, export settings)
   - Official disclosure template (ready to submit to Immunefi, C4, Sherlock)

---

## Installing the Tools

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

# Linux
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

### Semgrep
```bash
pip install semgrep
```

### cargo-audit
```bash
cargo install cargo-audit
```

### Clippy (comes with Rust)
```bash
rustup component add clippy
```

### solc
```bash
pip install solc-select
solc-select install latest
solc-select use latest
```

### Wake
```bash
pip install eth-wake
```

---

## CLI Reference

```
torot <path> [options]

Arguments:
  path                Path to the smart contracts or code folder

Options:
  --report FILE, -r   Output path for the Markdown report
  --api KEY=VALUE     API key (repeatable). Keys: openai, anthropic,
                      etherscan, github, github-repo
  --no-dashboard      Plain terminal output, no live TUI
  --concurrent N, -c  Max tools in parallel (default: 5)
  --list-tools        Show all tools and install status
  --version, -v       Show version
  --help, -h          Show help
```

---

## Architecture

```
torot/
  cli.py                   CLI entry point (argparse)
  core/
    engine.py              Async orchestration engine
    detector.py            File and language detection
    models.py              Data models (Bug, ScanSession, ApiConfig, ...)
    reproduction.py        PoC, Foundry test, video guide, disclosure template
    api_enricher.py        OpenAI, Claude, Etherscan, GitHub integrations
  scanners/
    base.py                BaseScanner (abstract — easy to extend)
    slither_scanner.py     Slither (full JSON parser)
    all_scanners.py        All other 16 tool integrations + registry
  tui/
    dashboard.py           Rich Live TUI dashboard
  report/
    generator.py           Markdown report writer
```

### Adding a New Tool

1. Open `torot/scanners/all_scanners.py`
2. Subclass `BaseScanner`, set `tool_name`, `binary_names`, implement `_run_tool()` and `_parse_output()`
3. Append the class to `ALL_SCANNERS` at the bottom of the file

---

## License

MIT

---

*Built for the blockchain security community.*
