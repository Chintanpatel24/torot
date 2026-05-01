

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

## UI — Warp-Style Split Terminal

```
┌────────────────────────────────────────────────────────────────────┐
│ TOROT  target: ./contracts/  C:2 H:5 M:8 L:3  total:18  ai:claude  │
├────────────────────────────────────────────────────────────────────┤
│ Live Output                                                        │
│                                                                    │
│  12:34:01  > Starting: slither                                     │
│  12:34:01  [slither] Reentrancy detected in Vault.sol:42           │
│  12:34:05  > Starting: mythril                                     │
│  12:34:06  [mythril] Integer overflow in Token.sol:67              │
│  12:34:10  [CRITICAL] [slither] Reentrancy-eth  Vault.sol:42       │
│  12:34:10  [HIGH]     [mythril] Integer Overflow Token.sol:67      │
│  12:34:15  > Finished: slither (14.2s)                             │
│  ...                                                               │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│ Agent Plan                                                         │
│   1. [+] Detect languages     (internal)                           │
│   2. [*] Static analysis      (all)          <- running            │
│   3. [o] AI review            (ai-analysis)  <- pending            │
│   4. [o] Generate report      (internal)                           │
├────────────────────────────────────────────────────────────────────┤
│ Conversation                                                       │
│  you   ./contracts/                                                │
│  torot Building attack plan for ./contracts/...                    │
├────────────────────────────────────────────────────────────────────┤
│  Step: Static analysis  |  Tool: all  |  Approve? [Approve] [Skip] │
├────────────────────────────────────────────────────────────────────┤
│ > Ask Torot anything, or type a target path...                     │
└────────────────────────────────────────────────────────────────────┘
```

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

## Architecture

```
torot/
  cli.py                    Entry point (argparse + asyncio)
  core/
    models.py               All data types (Session, Finding, AIConfig, ...)
    report.py               Markdown report generator
  ui/
    app.py                  Textual TUI (Warp-style split layout)
    wizard.py               Startup wizard (AI provider picker)
  agents/
    brain.py                AI reasoning (Claude / OpenAI / Ollama)
    controller.py           Main agent loop (drives everything)
    orchestrator.py         Tool runner + output parser
  tools/
    registry.py             39 tool definitions + generic runner
  memory/
    store.py                SQLite persistence + export
```
