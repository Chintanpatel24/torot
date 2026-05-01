#!/usr/bin/env python3
"""
Torot v2 — Universal Security Agent
CLI Entry Point

Usage:
  torot                              Interactive mode (wizard + TUI)
  torot <path>                       Scan a folder
  torot <address>                    Analyze a contract address
  torot "security question"          Ask the agent
  torot --update                     Update Torot + all installed tools
  torot --update --check             Check what has updates (no changes)
  torot --list-tools                 Show all tools + install status
  torot --history                    Show past scan sessions
  torot --no-ai                      Offline mode (tools still run)
  torot --api claude=sk-ant-...      Set API key directly
  torot --version
"""
from __future__ import annotations
import asyncio
import sys
import os
import argparse

from rich.console import Console

console = Console()


def parse_args():
    p = argparse.ArgumentParser(
        prog="torot",
        description="Torot v2 — Universal Security Agent",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
API keys (--api):
  claude=<key>           Anthropic Claude
  openai=<key>           OpenAI GPT-4
  ollama                 Use local Ollama
  ollama-model=<model>   Ollama model name (default: llama3)
  ollama-url=<url>       Ollama URL (default: http://localhost:11434)
  etherscan=<key>        On-chain contract fetch
  github=<token>         Auto-create GitHub issues
  github-repo=owner/repo GitHub target repo

Examples:
  torot                              # interactive wizard + TUI
  torot ./contracts/                 # scan a folder
  torot 0x1234...abcd                # analyze on-chain contract
  torot "is tx.origin safe?"         # ask a security question
  torot --api claude=sk-ant-...      # set Claude key
  torot --api ollama --api ollama-model=llama3
  torot --no-ai ./contracts/         # offline scan
  torot --update                     # update everything
  torot --update --check             # check for updates only
  torot --list-tools                 # see all 39 tools
  torot --history                    # past sessions
        """,
    )
    p.add_argument("target",
                   nargs="?",
                   help="Folder path, contract address, or security question")
    p.add_argument("--api",
                   action="append", metavar="KEY=VAL",
                   help="API config (repeatable)")
    p.add_argument("--no-ai",
                   action="store_true",
                   help="Skip AI, run in offline mode")
    p.add_argument("--update",
                   action="store_true",
                   help="Update Torot and all installed security tools")
    p.add_argument("--check",
                   action="store_true",
                   help="Used with --update: check for updates without installing")
    p.add_argument("--list-tools",
                   action="store_true",
                   help="Show all supported tools and their install status")
    p.add_argument("--history",
                   action="store_true",
                   help="Show past scan sessions from persistent memory")
    p.add_argument("--export",
                   metavar="SESSION_ID",
                   help="Export a past session to Markdown")
    p.add_argument("--concurrent", "-c",
                   type=int, default=5,
                   help="Max tools running in parallel (default: 5)")
    p.add_argument("--version", "-v",
                   action="store_true",
                   help="Show version and exit")
    return p.parse_args()


def build_ai_config(api_args: list[str] | None, no_ai: bool):
    from torot.core.models import AIConfig, AIProvider
    cfg = AIConfig()
    if no_ai:
        cfg.provider = AIProvider.NONE
        return cfg
    if not api_args:
        return None   # None = run startup wizard

    for entry in api_args:
        if "=" in entry:
            key, _, val = entry.partition("=")
        else:
            key, val = entry, ""
        key = key.strip().lower()
        val = val.strip()

        if   key in ("claude", "anthropic"): cfg.provider = AIProvider.CLAUDE;  cfg.api_key = val
        elif key == "openai":                cfg.provider = AIProvider.OPENAI;  cfg.api_key = val
        elif key == "ollama":                cfg.provider = AIProvider.OLLAMA
        elif key == "ollama-model":          cfg.ollama_model = val
        elif key == "ollama-url":            cfg.ollama_url   = val
        elif key == "etherscan":             cfg.etherscan_key = val
        elif key == "github":                cfg.github_token  = val
        elif key == "github-repo":           cfg.github_repo   = val
        else:
            console.print(f"  [dim]Unknown --api key ignored: {key}[/dim]")

    return cfg


# ─────────────────────────────────────────────────────────────────────────────
# Main async runner
# ─────────────────────────────────────────────────────────────────────────────

async def run(args):
    from torot.core.models       import Session, Domain
    from torot.memory.store      import MemoryStore
    from torot.agents.controller import TorotController
    from torot.ui.wizard         import (
        run_startup_wizard, print_tool_table,
        print_banner, print_memory_stats,
    )

    memory = MemoryStore()

    # ── --update ─────────────────────────────────────────────────────────────
    if args.update:
        from torot.core.updater import run_update
        run_update(check_only=args.check)
        return

    # ── --list-tools ─────────────────────────────────────────────────────────
    if args.list_tools:
        print_banner()
        print_tool_table()
        return

    # ── --history ─────────────────────────────────────────────────────────────
    if args.history:
        print_banner()
        print_memory_stats(memory)
        sessions = memory.get_recent_sessions(20)
        if not sessions:
            console.print("  No past sessions found.\n")
            return
        from rich.table import Table
        from rich import box
        import time
        tbl = Table(
            title="Past Sessions",
            box=box.SIMPLE_HEAVY,
            header_style="bold magenta",
        )
        tbl.add_column("ID",       width=15)
        tbl.add_column("When",     width=14)
        tbl.add_column("Target",   width=30)
        tbl.add_column("Domain",   width=12)
        tbl.add_column("Findings", justify="right", width=10)
        for s in sessions:
            ts = time.strftime("%m-%d %H:%M", time.localtime(s["start_time"]))
            tbl.add_row(
                s["id"], ts, s["target"][:30],
                s["domain"], str(s["total_findings"])
            )
        console.print(tbl)
        console.print()
        return

    # ── --export ──────────────────────────────────────────────────────────────
    if args.export:
        console.print(f"\n  [dim]Export for session {args.export} — "
                      "use Ctrl+E inside an active session.[/dim]\n")
        return

    # ── AI config ─────────────────────────────────────────────────────────────
    ai_config = build_ai_config(args.api, args.no_ai)
    if ai_config is None:
        ai_config = run_startup_wizard()

    # ── Build session ─────────────────────────────────────────────────────────
    target  = args.target or ""
    session = Session(target=target, ai_config=ai_config)

    # ── Launch TUI + agent ────────────────────────────────────────────────────
    controller = TorotController(session=session, memory=memory)
    app        = controller.create_app()

    async def agent_task():
        await asyncio.sleep(0.8)
        if target:
            controller._queue.put_nowait(target)
        await controller.run_loop()

    async with app.run_async():
        task = asyncio.create_task(agent_task())
        try:
            await task
        except (asyncio.CancelledError, KeyboardInterrupt):
            pass


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main():
    args = parse_args()

    if args.version:
        console.print("[bold cyan]Torot[/bold cyan] v2.0.0 — Universal Security Agent")
        sys.exit(0)

    try:
        asyncio.run(run(args))
    except KeyboardInterrupt:
        console.print("\n  Torot stopped.\n")
        sys.exit(0)


if __name__ == "__main__":
    main()
