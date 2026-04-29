"""
Optional API enrichment layer for Torot.
Runs only when the user provides API keys.

Supported:
  - OpenAI GPT-4         -- AI analysis + improved fix suggestions
  - Anthropic Claude     -- AI analysis + improved fix suggestions  
  - Etherscan            -- On-chain contract verification check
  - GitHub               -- Auto-open issues from findings
"""

from __future__ import annotations
import json
import urllib.request
import urllib.error
import urllib.parse
from typing import Callable, Optional

from torot.core.models import Bug, ApiConfig, ScanSession


# --------------------------------------------------------------------------- #
# AI enrichment (OpenAI or Anthropic)                                          #
# --------------------------------------------------------------------------- #

def _call_openai(prompt: str, api_key: str) -> str:
    payload = json.dumps({
        "model": "gpt-4o",
        "max_tokens": 600,
        "messages": [{"role": "user", "content": prompt}],
    }).encode()
    req = urllib.request.Request(
        "https://api.openai.com/v1/chat/completions",
        data=payload,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read())
    return data["choices"][0]["message"]["content"].strip()


def _call_anthropic(prompt: str, api_key: str) -> str:
    payload = json.dumps({
        "model": "claude-opus-4-6",
        "max_tokens": 600,
        "messages": [{"role": "user", "content": prompt}],
    }).encode()
    req = urllib.request.Request(
        "https://api.anthropic.com/v1/messages",
        data=payload,
        headers={
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read())
    return data["content"][0]["text"].strip()


def enrich_bug_with_ai(bug: Bug, config: ApiConfig, log: Optional[Callable] = None) -> None:
    """Add AI-powered analysis to a single bug. Modifies bug in-place."""
    prompt = (
        f"You are a smart contract security expert. Analyze this vulnerability concisely.\n\n"
        f"Tool: {bug.tool}\n"
        f"Title: {bug.title}\n"
        f"Severity: {bug.severity.value}\n"
        f"File: {bug.location}\n"
        f"Description: {bug.description}\n"
        f"Code: {bug.code_snippet[:400] if bug.code_snippet else 'N/A'}\n\n"
        f"In 3-5 sentences: (1) confirm whether this is a real vulnerability, "
        f"(2) describe the precise exploit scenario, (3) suggest the minimal code fix."
    )
    try:
        if config.anthropic_key:
            bug.ai_analysis = _call_anthropic(prompt, config.anthropic_key)
        elif config.openai_key:
            bug.ai_analysis = _call_openai(prompt, config.openai_key)
    except Exception as e:
        if log:
            log(f"AI enrichment failed for '{bug.title}': {e}")
        bug.ai_analysis = ""


# --------------------------------------------------------------------------- #
# Etherscan enrichment                                                         #
# --------------------------------------------------------------------------- #

def check_etherscan_verification(contract_address: str, api_key: str) -> dict:
    """Check if a contract is verified on Etherscan."""
    url = (
        f"https://api.etherscan.io/api"
        f"?module=contract&action=getsourcecode"
        f"&address={contract_address}&apikey={api_key}"
    )
    try:
        with urllib.request.urlopen(url, timeout=15) as resp:
            data = json.loads(resp.read())
        result = data.get("result", [{}])[0]
        return {
            "verified": bool(result.get("SourceCode")),
            "contract_name": result.get("ContractName", ""),
            "compiler": result.get("CompilerVersion", ""),
            "optimization": result.get("OptimizationUsed", ""),
        }
    except Exception:
        return {"verified": False, "error": "Etherscan lookup failed"}


def enrich_session_with_etherscan(session: ScanSession, log: Optional[Callable] = None) -> dict:
    """
    If any .sol files in the session look like deployed addresses,
    check their verification status. Returns a dict of results.
    """
    if not session.api_config or not session.api_config.has_etherscan():
        return {}

    results = {}
    for f in session.detected_files:
        if f.endswith(".sol"):
            # Check if the filename looks like an address placeholder
            name = f.split("/")[-1].replace(".sol", "")
            if name.startswith("0x") and len(name) == 42:
                if log:
                    log(f"Checking Etherscan for {name}...")
                results[name] = check_etherscan_verification(
                    name, session.api_config.etherscan_key
                )
    return results


# --------------------------------------------------------------------------- #
# GitHub issue creation                                                        #
# --------------------------------------------------------------------------- #

def create_github_issue(bug: Bug, config: ApiConfig, log: Optional[Callable] = None) -> str:
    """Create a GitHub issue for a bug. Returns the issue URL or error message."""
    if not config.has_github():
        return ""

    title = f"[{bug.severity.value}] {bug.title}"
    body = (
        f"**Severity:** {bug.severity.value}\n"
        f"**Tool:** `{bug.tool}`\n"
        f"**Location:** `{bug.location}`\n\n"
        f"## Description\n\n{bug.description}\n\n"
        f"## Code\n\n```solidity\n{bug.code_snippet}\n```\n\n"
        f"## Impact\n\n{bug.impact or 'See description.'}\n\n"
        f"## Fix\n\n{bug.fix_suggestion}\n\n"
        f"*Generated by [Torot](https://github.com/your-org/torot)*"
    )
    payload = json.dumps({"title": title, "body": body}).encode()
    req = urllib.request.Request(
        f"https://api.github.com/repos/{config.github_repo}/issues",
        data=payload,
        headers={
            "Authorization": f"token {config.github_token}",
            "Accept": "application/vnd.github.v3+json",
            "Content-Type": "application/json",
            "User-Agent": "torot-security-scanner",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            data = json.loads(resp.read())
        url = data.get("html_url", "")
        if log:
            log(f"GitHub issue created: {url}")
        return url
    except Exception as e:
        if log:
            log(f"GitHub issue creation failed: {e}")
        return ""


# --------------------------------------------------------------------------- #
# Main enrichment runner                                                       #
# --------------------------------------------------------------------------- #

def run_api_enrichment(
    session: ScanSession,
    log: Optional[Callable] = None,
) -> dict:
    """
    Run all enabled API enrichment passes on the session.
    Returns a summary dict.
    """
    cfg = session.api_config
    if not cfg:
        return {}

    summary = {"ai_enriched": 0, "github_issues": [], "etherscan": {}}
    all_bugs = session.all_bugs

    # AI enrichment — only for HIGH and CRITICAL by default (to save tokens)
    if cfg.has_ai():
        for bug in all_bugs:
            if bug.severity.value in ("CRITICAL", "HIGH", "MEDIUM"):
                enrich_bug_with_ai(bug, cfg, log)
                if bug.ai_analysis:
                    summary["ai_enriched"] += 1

    # Etherscan
    if cfg.has_etherscan():
        summary["etherscan"] = enrich_session_with_etherscan(session, log)

    # GitHub — create issues for CRITICAL and HIGH only
    if cfg.has_github():
        for bug in all_bugs:
            if bug.severity.value in ("CRITICAL", "HIGH"):
                url = create_github_issue(bug, cfg, log)
                if url:
                    summary["github_issues"].append(url)

    return summary
