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


# Blockchain & Smart Contract Security Scanner

An open-source, agent-style CLI tool that orchestrates 17 industry-standard
security analyzers, then produces a unified report with reproduction guides,
Foundry tests, PoC scripts, video recording instructions, and official
disclosure templates.

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
git clone https://github.com/Chintanpatel24/torot
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
