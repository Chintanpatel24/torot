"""
Torot Tool Registry
All supported security tools across all domains.
Each entry defines: tool_name, binary, domain, install hint, command template.
"""
from __future__ import annotations
import shutil
import asyncio
import time
from dataclasses import dataclass, field
from typing import Optional, Callable
from torot.core.models import Domain, Finding, Severity


@dataclass
class ToolDef:
    name:         str
    binary:       str
    domain:       Domain
    description:  str
    install_hint: str
    timeout:      int   = 300
    alt_binaries: list[str] = field(default_factory=list)

    def is_installed(self) -> bool:
        candidates = [self.binary] + self.alt_binaries
        return any(shutil.which(b) for b in candidates)

    def found_binary(self) -> Optional[str]:
        for b in [self.binary] + self.alt_binaries:
            p = shutil.which(b)
            if p:
                return p
        return None


# ─────────────────────────────────────────────────────────────────────────────
# All tools — add new ones here, they auto-appear everywhere
# ─────────────────────────────────────────────────────────────────────────────

ALL_TOOLS: list[ToolDef] = [

    # ── Blockchain / Solidity ─────────────────────────────────────────────
    ToolDef("slither",    "slither",    Domain.BLOCKCHAIN,
            "Static analysis — reentrancy, overflow, access control",
            "pip install slither-analyzer"),
    ToolDef("aderyn",     "aderyn",     Domain.BLOCKCHAIN,
            "Rust-based multi-contract Solidity analyzer",
            "cargo install aderyn"),
    ToolDef("mythril",    "myth",       Domain.BLOCKCHAIN,
            "Symbolic execution for EVM — reentrancy, tx.origin",
            "pip install mythril"),
    ToolDef("manticore",  "manticore",  Domain.BLOCKCHAIN,
            "Binary analysis via symbolic execution",
            "pip install manticore[native]"),
    ToolDef("echidna",    "echidna",    Domain.BLOCKCHAIN,
            "Property-based fuzzer for Solidity invariants",
            "brew install echidna  # macOS"),
    ToolDef("securify",   "securify",   Domain.BLOCKCHAIN,
            "Ethereum security pattern compliance checker",
            "pip install securify2", alt_binaries=["securify2"]),
    ToolDef("solhint",    "solhint",    Domain.BLOCKCHAIN,
            "Solidity linter — coding standards",
            "npm install -g solhint"),
    ToolDef("oyente",     "oyente",     Domain.BLOCKCHAIN,
            "Timestamp dependence and reentrancy detector",
            "pip install oyente"),
    ToolDef("smartcheck", "smartcheck", Domain.BLOCKCHAIN,
            "XPath-based vulnerability pattern detector",
            "npm install -g smartcheck"),
    ToolDef("halmos",     "halmos",     Domain.BLOCKCHAIN,
            "Bounded model checker — SMT-based proofs",
            "pip install halmos"),
    ToolDef("solc",       "solc",       Domain.BLOCKCHAIN,
            "Solidity compiler warnings and errors",
            "pip install solc-select && solc-select install latest"),
    ToolDef("wake",       "wake",       Domain.BLOCKCHAIN,
            "Python-based Solidity analysis framework",
            "pip install eth-wake"),
    ToolDef("4naly3er",   "4naly3er",   Domain.BLOCKCHAIN,
            "Audit-contest-style report generator",
            "npm install -g @4naly3er/cli", alt_binaries=["analyzer"]),
    ToolDef("pyrometer",  "pyrometer",  Domain.BLOCKCHAIN,
            "Range analysis for Solidity bounds errors",
            "cargo install pyrometer"),
    ToolDef("heimdall",   "heimdall",   Domain.BLOCKCHAIN,
            "EVM bytecode decompiler and analyzer",
            "cargo install heimdall-rs"),

    # ── Rust ─────────────────────────────────────────────────────────────
    ToolDef("cargo-audit","cargo-audit",Domain.BLOCKCHAIN,
            "Rust dependency vulnerability scanner",
            "cargo install cargo-audit"),
    ToolDef("clippy",     "cargo",      Domain.BLOCKCHAIN,
            "Rust linter — unsafe patterns and mistakes",
            "rustup component add clippy"),

    # ── Web App ───────────────────────────────────────────────────────────
    ToolDef("nuclei",     "nuclei",     Domain.WEBAPP,
            "Fast template-based vulnerability scanner",
            "go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"),
    ToolDef("nikto",      "nikto",      Domain.WEBAPP,
            "Web server scanner — misconfigs and vulnerabilities",
            "apt install nikto  # or brew install nikto"),
    ToolDef("sqlmap",     "sqlmap",     Domain.WEBAPP,
            "Automatic SQL injection detection and exploitation",
            "pip install sqlmap"),
    ToolDef("wfuzz",      "wfuzz",      Domain.WEBAPP,
            "Web application fuzzer — directories, params, headers",
            "pip install wfuzz"),
    ToolDef("ffuf",       "ffuf",       Domain.WEBAPP,
            "Fast web fuzzer — directory and parameter discovery",
            "go install github.com/ffuf/ffuf/v2@latest"),
    ToolDef("gobuster",   "gobuster",   Domain.WEBAPP,
            "Directory and DNS brute-force tool",
            "go install github.com/OJ/gobuster/v3@latest"),
    ToolDef("whatweb",    "whatweb",    Domain.WEBAPP,
            "Web technology fingerprinting",
            "gem install whatweb"),
    ToolDef("dalfox",     "dalfox",     Domain.WEBAPP,
            "XSS scanner and parameter analyzer",
            "go install github.com/hahwul/dalfox/v2@latest"),
    ToolDef("semgrep",    "semgrep",    Domain.WEBAPP,
            "Pattern-based static analysis (all languages)",
            "pip install semgrep"),
    ToolDef("trufflehog", "trufflehog", Domain.WEBAPP,
            "Secret and credential scanner",
            "go install github.com/trufflesecurity/trufflehog/v3@latest"),
    ToolDef("gitleaks",   "gitleaks",   Domain.WEBAPP,
            "Git history secret scanner",
            "brew install gitleaks  # or go install"),

    # ── API Security ──────────────────────────────────────────────────────
    ToolDef("arjun",      "arjun",      Domain.API,
            "HTTP parameter discovery tool",
            "pip install arjun"),
    ToolDef("jwt_tool",   "jwt_tool",   Domain.API,
            "JWT analysis and attack toolkit",
            "pip install jwt_tool", alt_binaries=["jwt-tool"]),
    ToolDef("kiterunner", "kr",         Domain.API,
            "API route brute-force and discovery",
            "go install github.com/assetnote/kiterunner/cmd/kr@latest"),
    ToolDef("graphqlmap", "graphqlmap", Domain.API,
            "GraphQL endpoint mapper and injection tester",
            "pip install graphqlmap"),

    # ── Binary / Native ───────────────────────────────────────────────────
    ToolDef("radare2",    "r2",         Domain.BINARY,
            "Reverse engineering framework",
            "brew install radare2  # or apt install radare2"),
    ToolDef("binwalk",    "binwalk",    Domain.BINARY,
            "Firmware analysis and extraction",
            "pip install binwalk"),
    ToolDef("checksec",   "checksec",   Domain.BINARY,
            "Binary security feature checker",
            "pip install checksec"),
    ToolDef("strings",    "strings",    Domain.BINARY,
            "Extract printable strings from binaries",
            "Usually pre-installed on Linux/macOS"),
    ToolDef("objdump",    "objdump",    Domain.BINARY,
            "Binary disassembler and analyzer",
            "Usually pre-installed (binutils)"),
    ToolDef("ltrace",     "ltrace",     Domain.BINARY,
            "Library call tracer for binaries",
            "apt install ltrace"),
    ToolDef("strace",     "strace",     Domain.BINARY,
            "System call tracer",
            "apt install strace"),
]

TOOL_MAP: dict[str, ToolDef] = {t.name: t for t in ALL_TOOLS}


def get_installed_tools() -> list[ToolDef]:
    return [t for t in ALL_TOOLS if t.is_installed()]


def get_tools_for_domain(domain: Domain) -> list[ToolDef]:
    return [t for t in ALL_TOOLS if t.domain == domain]


def get_installed_for_domain(domain: Domain) -> list[ToolDef]:
    return [t for t in get_tools_for_domain(domain) if t.is_installed()]


# ─────────────────────────────────────────────────────────────────────────────
# Generic tool runner
# ─────────────────────────────────────────────────────────────────────────────

async def run_tool(
    tool:        ToolDef,
    args:        list[str],
    cwd:         str = ".",
    on_line:     Optional[Callable[[str], None]] = None,
    timeout:     int = 0,
) -> tuple[str, str, float]:
    """
    Run a tool asynchronously.
    Streams output line-by-line via on_line callback.
    Returns (stdout, stderr, duration).
    """
    binary = tool.found_binary()
    if not binary:
        return "", f"{tool.name} not installed", 0.0

    cmd = [binary] + args
    t0  = time.time()

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        cwd=cwd,
    )

    stdout_lines: list[str] = []
    stderr_lines: list[str] = []

    async def read_stream(stream, collector, label):
        async for raw_line in stream:
            line = raw_line.decode("utf-8", errors="replace").rstrip()
            collector.append(line)
            if on_line:
                on_line(f"[{tool.name}] {line}")

    to = timeout or tool.timeout
    try:
        await asyncio.wait_for(
            asyncio.gather(
                read_stream(proc.stdout, stdout_lines, "out"),
                read_stream(proc.stderr, stderr_lines, "err"),
            ),
            timeout=to,
        )
    except asyncio.TimeoutError:
        proc.kill()
        stderr_lines.append(f"[timeout after {to}s]")

    await proc.wait()
    duration = time.time() - t0
    return "\n".join(stdout_lines), "\n".join(stderr_lines), duration
