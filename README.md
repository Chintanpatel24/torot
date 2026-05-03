<div align=center>

<img width="250" alt="torot" src="https://github.com/user-attachments/assets/3d9d5832-a9eb-4bef-be3a-2d13f2e0a610" />

![Torot](https://img.shields.io/badge/version-2.0.0-amber?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue?style=flat-square)

</div>

# Torot - Universal Agent for bug bounty & ...
- An open-source, agent-style CLI security tool that thinks and acts like an elite
security researcher. It orchestrates 39 industry tools across blockchain, web app,
binary, and API security - with a live Warp-style split terminal UI, persistent
memory, and AI reasoning via Claude, GPT-4, or Ollama.


---

## What Torot Does !!

Torot is a security research agent that:

1. **Accepts any input**
   >- folder path, contract address, or a plain security question

2. **Detects the domain**
   >- blockchain, web app, binary, or API automatically
3. **Builds an attack plan**
   >- AI generates a step-by-step assessment plan
4. **Asks your approval**
   >- semi-auto mode: you approve or skip each step
5. **Runs all tools in parallel**
   >- every installed tool fires concurrently
6. **Streams everything live**
   >- top pane shows all tool output in real time
7. **Analyses findings with AI**
   >- brain reviews each finding, writes PoC and disclosure
8. **Persists to memory**
   >- SQLite database remembers every session and finding
9. **Exports full reports**
    >- Markdown with reproduction guides, Foundry tests, video guides

---

## Quick Start

```bash
# Install
pip install torot

# Or from source
git clone https://github.com/Chintanpatel24/torot
cd torot
pip install -e .

# Launch interactive mode (wizard picks AI provider)
torot

# Direct scan — skips wizard
torot ./my-contracts/

# Analyze an on-chain contract
torot 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48

# Ask a security question
torot "is tx.origin safe for authentication?"

# Offline mode (no AI, tools still run)
torot --no-ai ./my-contracts/
```

---

## For more : [visit usage.md](usage.md)
