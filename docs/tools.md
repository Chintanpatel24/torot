# Supported Tools

Torot auto-detects and orchestrates these security tools:

## Web Application
| Tool       | Domain   | Purpose                          | Install                        |
|------------|----------|----------------------------------|--------------------------------|
| nmap       | webapp   | Port scanning, service detection | `apt install nmap`             |
| nuclei     | webapp   | Template-based vulnerability scan| `go install .../nuclei@latest` |
| httpx      | webapp   | HTTP probing                     | `go install .../httpx@latest`  |
| katana     | webapp   | Web crawling                     | `go install .../katana@latest` |
| ffuf       | webapp   | Directory fuzzing                | `go install .../ffuf@latest`   |
| gobuster   | webapp   | Directory brute force            | `go install .../gobuster@latest`|
| nikto      | webapp   | Web server baseline check        | `apt install nikto`            |

## Subdomain / Recon
| Tool       | Domain   | Purpose                          | Install                        |
|------------|----------|----------------------------------|--------------------------------|
| bbot       | webapp   | Asset discovery automation       | `pipx install bbot`            |
| subfinder  | webapp   | Passive subdomain enumeration    | `go install .../subfinder@latest`|
| amass      | webapp   | DNS intelligence                 | `go install .../amass@latest`  |

## API / Injection
| Tool       | Domain   | Purpose                          | Install                        |
|------------|----------|----------------------------------|--------------------------------|
| sqlmap     | api      | SQL injection verification       | `pipx install sqlmap`          |

## Code Analysis / Secrets
| Tool       | Domain   | Purpose                          | Install                        |
|------------|----------|----------------------------------|--------------------------------|
| semgrep    | general  | Static analysis                  | `pipx install semgrep`         |
| trufflehog | general  | Secret discovery                 | `go install .../trufflehog@latest`|
| gitleaks   | general  | Git secret scanning             | `go install .../gitleaks@latest`|

## Adding Custom Tools

Tools are defined in `~/.torot/config.json`. You can add custom
profiles with:
```json
{
  "name": "my-tool",
  "domain": "webapp",
  "binary_names": ["my-tool"],
  "args": ["--target", "{{target_url}}"],
  "output_format": "json",
  "input_kinds": ["url"]
}
```
