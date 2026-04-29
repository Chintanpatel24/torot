"""
Torot core data models.
"""

from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional
import time


class Severity(str, Enum):
    CRITICAL = "CRITICAL"
    HIGH     = "HIGH"
    MEDIUM   = "MEDIUM"
    LOW      = "LOW"
    INFO     = "INFO"

    @property
    def color(self) -> str:
        return {
            "CRITICAL": "bold red",
            "HIGH":     "red",
            "MEDIUM":   "yellow",
            "LOW":      "cyan",
            "INFO":     "dim white",
        }[self.value]

    @property
    def order(self) -> int:
        return {"CRITICAL": 0, "HIGH": 1, "MEDIUM": 2, "LOW": 3, "INFO": 4}[self.value]

    @property
    def marker(self) -> str:
        return {"CRITICAL": "[C]", "HIGH": "[H]", "MEDIUM": "[M]", "LOW": "[L]", "INFO": "[I]"}[self.value]


class ToolStatus(str, Enum):
    PENDING       = "pending"
    CHECKING      = "checking"
    NOT_INSTALLED = "not_installed"
    RUNNING       = "running"
    COMPLETED     = "completed"
    FAILED        = "failed"
    SKIPPED       = "skipped"

    @property
    def color(self) -> str:
        return {
            "pending":       "dim",
            "checking":      "blue",
            "not_installed": "dim red",
            "running":       "bold yellow",
            "completed":     "bold green",
            "failed":        "bold red",
            "skipped":       "dim yellow",
        }[self.value]

    @property
    def icon(self) -> str:
        return {
            "pending":       "o",
            "checking":      "~",
            "not_installed": "x",
            "running":       "*",
            "completed":     "+",
            "failed":        "!",
            "skipped":       "-",
        }[self.value]


@dataclass
class ReproductionGuide:
    steps: list[str]            = field(default_factory=list)
    poc_script: str             = ""
    foundry_test: str           = ""
    video_guide: str            = ""
    disclosure_template: str    = ""
    environment_setup: str      = ""
    expected_output: str        = ""


@dataclass
class Bug:
    tool:            str
    title:           str
    severity:        Severity
    description:     str
    file:            str                       = ""
    line:            int                       = 0
    code_snippet:    str                       = ""
    fix_suggestion:  str                       = ""
    impact:          str                       = ""
    references:      list[str]                 = field(default_factory=list)
    bug_type:        str                       = ""
    raw:             str                       = ""
    reproduction:    Optional[ReproductionGuide] = None
    ai_analysis:     str                       = ""
    production_path: str                       = ""

    @property
    def location(self) -> str:
        if self.file and self.line:
            return f"{self.file}:{self.line}"
        elif self.file:
            return self.file
        return "unknown"


@dataclass
class ToolResult:
    tool_name:  str
    status:     ToolStatus
    bugs:       list[Bug] = field(default_factory=list)
    raw_output: str       = ""
    error:      str       = ""
    duration:   float     = 0.0
    start_time: float     = field(default_factory=time.time)

    @property
    def bug_counts(self) -> dict[str, int]:
        counts = {s.value: 0 for s in Severity}
        for bug in self.bugs:
            counts[bug.severity.value] += 1
        return counts


@dataclass
class ApiConfig:
    openai_key:    str           = ""
    anthropic_key: str           = ""
    etherscan_key: str           = ""
    github_token:  str           = ""
    github_repo:   str           = ""
    custom_apis:   dict[str,str] = field(default_factory=dict)

    def has_ai(self) -> bool:
        return bool(self.openai_key or self.anthropic_key)

    def has_etherscan(self) -> bool:
        return bool(self.etherscan_key)

    def has_github(self) -> bool:
        return bool(self.github_token and self.github_repo)


@dataclass
class ScanSession:
    target_path:        str
    start_time:         float                  = field(default_factory=time.time)
    end_time:           float                  = 0.0
    tool_results:       dict[str,ToolResult]   = field(default_factory=dict)
    detected_languages: list[str]              = field(default_factory=list)
    detected_files:     list[str]              = field(default_factory=list)
    api_config:         Optional[ApiConfig]    = None

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

    @property
    def tools_ran(self) -> int:
        return sum(
            1 for r in self.tool_results.values()
            if r.status in (ToolStatus.COMPLETED, ToolStatus.FAILED)
        )

    @property
    def tools_available(self) -> int:
        return sum(
            1 for r in self.tool_results.values()
            if r.status != ToolStatus.NOT_INSTALLED
        )
