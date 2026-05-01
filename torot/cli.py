#!/usr/bin/env python3
"""
Torot v2 — Universal Security Agent
CLI Entry Point

Usage:
  torot                           Interactive mode (wizard + TUI)
  torot <path>                    Scan a folder directly
  torot <address>                 Analyze a contract address
  torot --list-tools              Show all tools and install status
  torot --history                 Show past sessions
  torot --export <session_id>     Export a past session
  torot --no-ai                   Skip AI wizard, offline mode
  torot --api claude=sk-ant-...   Set API key directly
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
Examples:
  torot                              # interactive wizard + TUI
  torot ./contracts/                 # scan a folder
  torot 0x1234...abcd                # analyze on-chain contract
  torot --no-ai ./contracts/         # offline scan, no AI
  torot --api claude=sk-ant-...      # set Claude key
  torot --api ollama --api ollama-model=llama3
  torot --list-tools                 # show all tools
  torot --history                    # show past sessions
        """,
    )
    p.add_argument("target",         nargs="?",      help="Folder path, contract address, or question")
    p.add_argument("--api",          action="append",metavar="KEY=VAL",
                   help="API config (repeatable): claude=KEY, openai=KEY, etherscan=KEY, github=TOKEN, github-repo=owner/repo, ollama, ollama-model=MODEL, ollama-url=URL")
    p.add_argument("--no-ai",        action="store_true", help="Skip AI, run in offline mode")
    p.add_argument("--list-tools",   action="store_true", help="Show all tools and install status")
    p.add_argument("--history",      action="store_true", help="Show past scan sessions")
    p.add_argument("--export",       metavar="SESSION_ID",  help="Export a past session to Markdown")
    p.add_argument("--concurrent","-c", type=int, default=5, help="Max parallel tools (default: 5)")
    p.add_argument("--version","-v", action="store_true", help="Show version")
    return p.parse_args()


def build_ai_config_from_args(api_args: list[str] | None, no_ai: bool):
    from torot.core.models import AIConfig, AIProvider
    cfg = AIConfig()
    if no_ai:
        cfg.provider = AIProvider.NONE
        return cfg
    if not api_args:
        return None   # None = run wizard

    for entry in (api_args or []):
        if "=" in entry:
            key, _, val = entry.partition("=")
            key = key.strip().lower()
            val = val.strip()
        else:
            key = entry.strip().lower()
            val = ""

        if   key == "claude":       cfg.provider = AIProvider.CLAUDE;  cfg.api_key = val
        elif key == "anthropic":    cfg.provider = AIProvider.CLAUDE;  cfg.api_key = val
        elif key == "openai":       cfg.provider = AIProvider.OPENAI;  cfg.api_key = val
        elif key == "ollama":       cfg.provider = AIProvider.OLLAMA
        elif key == "ollama-model": cfg.ollama_model = val
        elif key == "ollama-url":   cfg.ollama_url   = val
        elif key == "etherscan":    cfg.etherscan_key = val
        elif key == "github":       cfg.github_token  = val
        elif key == "github-repo":  cfg.github_repo   = val

    return cfg


async def run(args):
    from torot.core.models       import Session, InputMode, Domain
    from torot.memory.store      import MemoryStore
    from torot.agents.controller import TorotController
    from torot.ui.wizard         import run_startup_wizard, print_banner

    memory = MemoryStore()

    # ── list-tools ────────────────────────────────────────────────────────
    if args.list_tools:
        from torot.ui.wizard import print_tool_table, print_banner
        print_banner()
        print_tool_table()
        return

    # ── history ───────────────────────────────────────────────────────────
    if args.history:
        from torot.ui.wizard import print_banner
        print_banner()
        sessions = memory.get_recent_sessions(20)
        if not sessions:
            console.print("  No past sessions found.\n")
            return
        from rich.table import Table
        from rich import box
        tbl = Table(title="Past Sessions", box=box.SIMPLE_HEAVY, header_style="bold magenta")
        tbl.add_column("ID",       width=14)
        tbl.add_column("Target",   width=30)
        tbl.add_column("Domain",   width=12)
        tbl.add_column("Findings", justify="right", width=10)
        import time
        for s in sessions:
            ts = time.strftime("%m-%d %H:%M", time.localtime(s["start_time"]))
            tbl.add_row(s["id"], s["target"][:30], s["domain"], str(s["total_findings"]))
        console.print(tbl)
        console.print()
        return

    # ── export ────────────────────────────────────────────────────────────
    if args.export:
        console.print(f"  Export not yet available for session {args.export}.")
        console.print("  Use Ctrl+E inside an active session to export.\n")
        return

    # ── AI config ─────────────────────────────────────────────────────────
    ai_config = build_ai_config_from_args(args.api, args.no_ai)
    if ai_config is None:
        # No --api flags given and not --no-ai: run the wizard
        ai_config = run_startup_wizard()

    # ── Build session ──────────────────────────────────────────────────────
    target    = args.target or ""
    session   = Session(
        target=target,
        ai_config=ai_config,
    )

    # ── Launch controller + TUI ────────────────────────────────────────────
    controller = TorotController(session=session, memory=memory)
    app        = controller.create_app()

    # Run TUI and agent loop concurrently
    async def agent_task():
        # Give TUI a moment to mount
        await asyncio.sleep(0.8)
        # If a target was given on CLI, inject it immediately
        if target:
            controller._queue.put_nowait(target)
        await controller.run_loop()

    # Run the Textual app; agent loop runs as a background task
    async with app.run_async():
        task = asyncio.create_task(agent_task())
        try:
            await task
        except asyncio.CancelledError:
            pass


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
