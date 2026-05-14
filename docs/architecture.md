# Torot Architecture

## Overview

Torot is an autonomous bug bounty orchestration platform. It detects
security tools installed on your system, orchestrates them in parallel
against a target, and generates Markdown reports with parsed findings.

## Module Map

```
torot/
├── app/            CLI and TUI entry points
│   ├── cli.rs      Command-line argument parsing and dispatch
│   └── tui.rs      Terminal UI bootstrap
├── core/           Core engine and data model
│   ├── types.rs    All data structures (Finding, Session, ToolProfile, etc.)
│   ├── config.rs   Configuration loading, saving, built-in tool profiles
│   ├── state.rs    Application state (AppState, DB queries)
│   ├── db.rs       SQLite schema and CRUD operations
│   ├── tools.rs    Tool detection, argument rendering, target inference
│   ├── engine.rs   Pipeline execution, parallel tool runs, streaming
│   ├── parser.rs   Output parsing (JSON, JSONL, text) for each tool
│   ├── report.rs   Markdown report generation
│   ├── event.rs    Event bus for async communication
│   ├── sandbox.rs  Sandbox profile definitions
│   └── knowledge.rs Knowledge topics and descriptions
├── swarm/          Task orchestration with circuit breakers
│   ├── coordinator.rs  QueenCoordinator — manages execution waves
│   ├── circuit_breaker.rs  Failure detection and prevention
│   ├── planner.rs   Dependency-wave execution planning
│   └── executor.rs  Task execution and output collection
├── tui/            Terminal UI (ratatui)
│   ├── app.rs      Main event loop and state machine
│   ├── theme.rs    Colors and style constants
│   ├── widgets.rs  Reusable UI components
│   └── views/      Screen implementations
│       ├── home.rs      Target input, mode, tool selection
│       ├── scan.rs      Live output stream + findings tab
│       ├── findings.rs  Finding list + detail view
│       ├── history.rs   Past sessions table
│       ├── tools.rs     Tool registry display
│       └── settings.rs  Configuration editor
└── util/           Shared utilities
    ├── time.rs     Timestamp formatting
    ├── path.rs     Path resolution
    └── fmt.rs      String formatting helpers
```

## Data Flow

1. User inputs target + selects tools (TUI or CLI)
2. `start_scan()` creates a Session and kicks off the pipeline
3. Pipeline spawns one tokio task per tool, all running in parallel
4. Each tool process has its stdout/stderr streamed via the EventBus
5. The EventBus feeds lines to the TUI (or is ignored in CLI mode)
6. On completion, output is parsed for findings, stored in SQLite
7. A Markdown report is generated and written to `~/.torot/reports/`

## Event Bus

The EventBus uses `tokio::sync::broadcast` for one-to-many
communication between the pipeline and the TUI. Events include:
- `Line` — raw output line from a tool
- `Finding` — a parsed finding
- `ScanComplete` — signals scan end

## Database

SQLite at `~/.torot/memory.db` with tables:
- `sessions` — scan sessions with timing and summary
- `findings` — parsed findings linked to sessions
