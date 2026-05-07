<div align=center>
  
 <pre>

 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║   
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   

</pre>

</div>

---

## AI Providers

At startup, Torot shows a provider picker:

```
1  Anthropic Claude    claude-opus-4-6  (recommended)
2  OpenAI GPT-4        gpt-4o
3  Ollama (local)      runs 100% offline — llama3, mistral, etc.
4  None / offline      tools run, no AI reasoning
```

Or skip the wizard with flags:

```bash
# Claude
torot --api claude=sk-ant-...

# OpenAI
torot --api openai=sk-...

# Ollama (local, no internet needed)
torot --api ollama --api ollama-model=llama3

# Etherscan + GitHub
torot --api etherscan=ABC123 --api github=ghp_TOKEN --api github-repo=owner/repo
```

---

## Report templates

Reports use simple `{{placeholder}}` syntax:

| Placeholder | Description |
|-------------|-------------|
| `{{session_id}}` | Unique session ID |
| `{{target}}` | Scan target |
| `{{created_at}}` | Unix timestamp |
| `{{findings_total}}` | Total finding count |
| `{{critical_count}}` | Critical findings |
| `{{high_count}}` | High findings |
| `{{summary}}` | Auto-generated summary |
| `{{tool_overview}}` | Per-tool finding counts |
| `{{findings_table}}` | Markdown table of all findings |

---

**Keyboard shortcuts:**
- `Ctrl+C` - quit
- `Ctrl+L` - clear log
- `Ctrl+E` - export report
- `Ctrl+P` - show/refresh plan
- `Escape` - cancel/skip current step

---

## Input Modes

| Input | Example | What happens |
|-------|---------|--------------|
| Folder path | `./contracts/` | Detects languages, runs all tools, full report |
| Contract address | `0xA0b86...` | Fetches from Etherscan, decompiles, scans |
| Question | `is delegatecall safe?` | AI answers with memory context |

---

## 39 Supported Tools

Run `torot --list-tools` to see which are installed.

### Blockchain (17 tools)
slither, aderyn, mythril, manticore, echidna, securify, solhint, oyente,
smartcheck, halmos, solc, wake, 4naly3er, pyrometer, heimdall, cargo-audit, clippy

### Web App (11 tools)
nuclei, nikto, sqlmap, wfuzz, ffuf, gobuster, whatweb, dalfox, semgrep, trufflehog, gitleaks

### API Security (4 tools)
arjun, jwt_tool, kiterunner, graphqlmap

### Binary / Native (7 tools)
radare2, binwalk, checksec, strings, objdump, ltrace, strace

---

## Memory System

Torot persists everything to `~/.torot/memory.db`:

```bash
# View past sessions
torot --history

# Export a session
torot --export <session-id>
```

Knowledge from every session is indexed and used to answer future questions.

---

## Adding a New Tool

1. Open `torot/tools/registry.py`
2. Add a `ToolDef(...)` entry to `ALL_TOOLS`
3. Add a parser in `torot/agents/orchestrator.py` → `PARSERS` dict
4. Add command args in `_build_args()`

That's it — the tool auto-appears in `--list-tools`, the wizard, and all scan flows.

---

## Building

### Prerequisites
- Rust 1.77+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Node.js 18+ (`https://nodejs.org`)
- System deps (Linux): `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev`

### Development
```bash
npm install
npm run tauri:dev
```

### Production build
```bash
npm run tauri:build
# Output: src-tauri/target/release/bundle/
```

### Icon generation (requires ImageMagick)
```bash
chmod +x scripts/gen-icons.sh
./scripts/gen-icons.sh assets/torot-logo.png
```

---

## Supported tools — 28 total

### Blockchain (13)
`slither` `aderyn` `mythril` `echidna` `manticore` `solhint` `halmos`
`semgrep` `solc` `wake` `heimdall` `cargo-audit` `clippy`

### Web App (7)
`nuclei` `nikto` `sqlmap` `ffuf` `gobuster` `dalfox` `trufflehog` `gitleaks`

### API (2)
`arjun` `jwt_tool`

### Binary (6)
`radare2` `binwalk` `checksec` `strings` `objdump` `ltrace`

---

## Built-in rules engine

16 pre-loaded security rules covering:

| ID | Severity | Description |
|----|----------|-------------|
| SOL-001 | CRITICAL | Reentrancy |
| SOL-002 | HIGH | tx.origin authentication |
| SOL-003 | HIGH | Integer overflow |
| SOL-004 | CRITICAL | Unprotected selfdestruct |
| SOL-005 | MEDIUM | Timestamp dependence |
| SOL-006 | HIGH | Flash loan attack surface |
| SOL-007 | HIGH | Missing access control |
| WEB-001 | CRITICAL | SQL injection |
| WEB-002 | HIGH | XSS |
| WEB-003 | HIGH | SSRF |
| WEB-004 | CRITICAL | Hardcoded credentials |
| BIN-001 | CRITICAL | Stack buffer overflow |
| BIN-002 | MEDIUM | Missing binary hardening |
| API-001 | HIGH | IDOR/BOLA |
| API-002 | HIGH | JWT vulnerability |

Custom rules can be loaded as JSON at runtime.

---

## Swarm agent architecture

The TypeScript swarm engine (`src/agents/swarm.ts`) implements:

- **QueenCoordinator** — hierarchical task orchestration
- **CircuitBreaker** — per-tool failure isolation (3 strikes → open 30s)
- **Dependency waves** — tasks run in phases respecting `dependsOn` chains
- **Priority scheduling** — recon (10) → static (8) → dynamic (6) → binary (4)
- **Memory store** — key-value store for inter-task communication
- **Parallel execution** — configurable `maxParallel` with semaphore

---

## Update

```bash
./update.sh                # update everything
./update.sh --torot-only   # only update Torot
./update.sh --tools-only   # only update security tools
./update.sh --check        # check versions, no changes
```

---

## Run modes

| Mode | Behaviour |
|------|-----------|
| Single | Scan once, display results |
| Loop | Repeat scan until stopped (useful for CI/watch) |
| Daemon | Watch a folder for file changes, rescan automatically |

---

## License

MIT
