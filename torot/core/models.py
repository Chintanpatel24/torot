"""
Core data models for Torot.
"""

from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional
import time


class Severity(str, Enum):
    CRITICAL = "CRITICAL"
    HIGH = "HIGH"
    MEDIUM = "MEDIUM"
    LOW = "LOW"
    INFO = "INFO"

    @property
    def color(self) -> str:
        return {
            "CRITICAL": "bold red",
            "HIGH": "red",
            "MEDIUM": "yellow",
            "LOW": "cyan",
            "INFO": "dim white",
        }[self.value]

    @property
    def emoji(self) -> str:
        return {
            "CRITICAL": "💀",
            "HIGH": "🔴",
            "MEDIUM": "🟡",
            "LOW": "🔵",
            "INFO": "⚪",
        }[self.value]

    @property
    def order(self) -> int:
        return {"CRITICAL": 0, "HIGH": 1, "MEDIUM": 2, "LOW": 3, "INFO": 4}[self.value]


class ToolStatus(str, Enum):
    PENDING = "pending"
    CHECKING = "checking"     # checking if tool is installed
    NOT_INSTALLED = "not_installed"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    SKIPPED = "skipped"

    @property
    def color(self) -> str:
        return {
            "pending": "dim",
            "checking": "blue",
            "not_installed": "dim red",
            "running": "bold yellow",
            "completed": "bold green",
            "failed": "bold red",
            "skipped": "dim yellow",
        }[self.value]

    @property
    def icon(self) -> str:
        return {
            "pending": "○",
            "checking": "⟳",
            "not_installed": "✗",
            "running": "◉",
            "completed": "✔",
            "failed": "✘",
            "skipped": "⊘",
        }[self.value]


@dataclass
class Bug:
    tool: str
    title: str
    severity: Severity
    description: str
    file: str = ""
    line: int = 0
    code_snippet: str = ""
    fix_suggestion: str = ""
    impact: str = ""
    references: list[str] = field(default_factory=list)
    bug_type: str = ""
    raw: str = ""

    @property
    def location(self) -> str:
        if self.file and self.line:
            return f"{self.file}:{self.line}"
        elif self.file:
            return self.file
        return "unknown"


@dataclass
class ToolResult:
    tool_name: str
    status: ToolStatus
    bugs: list[Bug] = field(default_factory=list)
    raw_output: str = ""
    error: str = ""
    duration: float = 0.0
    start_time: float = field(default_factory=time.time)

    @property
    def bug_counts(self) -> dict[str, int]:
        counts = {s.value: 0 for s in Severity}
        for bug in self.bugs:
            counts[bug.severity.value] += 1
        return counts


@dataclass
class ScanSession:
    target_path: str
    start_time: float = field(default_factory=time.time)
    end_time: float = 0.0
    tool_results: dict[str, ToolResult] = field(default_factory=dict)
    detected_languages: list[str] = field(default_factory=list)
    detected_files: list[str] = field(default_factory=list)

    @property
    def all_bugs(self) -> list[Bug]:
        bugs = []
        for result in self.tool_results.values():
            bugs.extend(result.bugs)
        return sorted(bugs, key=lambda b: b.severity.order)

    @property
    def total_bugs(self) -> int:
        return len(self.all_bugs)

    @property
    def duration(self) -> float:
        end = self.end_time if self.end_time else time.time()
        return end - self.start_time

    @property
    def bug_summary(self) -> dict[str, int]:
        counts = {s.value: 0 for s in Severity}
        for bug in self.all_bugs:
            counts[bug.severity.value] += 1
        return counts
