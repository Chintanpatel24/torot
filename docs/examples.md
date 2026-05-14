# Examples

## CLI Usage

```bash
# List detected tools
torot tools

# Quick scan with auto-detected tools
torot scan --target https://example.com

# Scan with specific tools
torot scan --target https://example.com --tools nuclei,httpx

# Deep scan with custom report output
torot scan --target https://example.com --mode deep --output ./report.md

# Scan code repository
torot scan --target /path/to/repo --tools semgrep,gitleaks

# Generate report from a previous session
torot report --session abc123 --output ./report.md

# Show current configuration
torot config
```

## TUI Usage

Launch the TUI by running `torot` with no arguments:

```
1 HOME
2 SCAN      ┌──────────────────────────────────────────┐
3 FINDINGS  │  TOROT v4                                │
4 HISTORY   │                                          │
5 TOOLS     │  Target: https://example.com              │
6 SETTINGS  │  Mode: single   [1/2/3 to change]        │
            │  Tools: auto-detect                      │
            │  [x]nmap  [x]nuclei  [x]httpx            │
            │                                          │
            │  [Enter] Launch scan                     │
            └──────────────────────────────────────────┘
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1`-`6` | Switch views (Home, Scan, Findings, History, Tools, Settings) |
| `Tab` / `←` `→` | Cycle through views |
| `Ctrl+q` | Quit |
| `Esc` | Go to Home |
| `/` | Enter edit mode (Home target, Tools search) |
| `Enter` | Launch scan (Home) / Confirm edit |
| `Space` | Toggle all tools (Home) |
| `a` | Toggle advanced options (Home) |
| `↑` `↓` `PgUp` `PgDn` | Scroll output (Scan view) |
| `r` | Reset scroll to bottom |
| `t` / `Tab` | Toggle Output/Findings tabs |
| `s` | Stop running scan |
| `e` | Export report when scan is done |

## Report Template

You can customize the report template in settings or via CLI:

```bash
torot scan --target https://example.com \
  --template-file my-template.md \
  --output custom-report.md
```

Available placeholders:
- `{{session_id}}` — unique session identifier
- `{{target}}` — scan target
- `{{created_at}}` — Unix timestamp
- `{{findings_total}}` — total finding count
- `{{critical_count}}` — critical severity count
- `{{high_count}}` — high severity count
- `{{summary}}` — human-readable summary
- `{{tool_overview}}` — tool-coverage breakdown
- `{{findings_table}}` — markdown findings table
