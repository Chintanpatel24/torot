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

### Adding a New Tool

1. Open `torot/scanners/all_scanners.py`
2. Subclass `BaseScanner`, set `tool_name`, `binary_names`, implement `_run_tool()` and `_parse_output()`
3. Append the class to `ALL_SCANNERS` at the bottom of the file
