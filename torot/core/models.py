"""
Torot v2 — Core Data Models
Universal Security Agent
"""
from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, Any
import time
import uuid


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


class Domain(str, Enum):
    BLOCKCHAIN = "blockchain"
    WEBAPP     = "webapp"
    BINARY     = "binary"
    API        = "api"
    GENERAL    = "general"


class InputMode(str, Enum):
    FOLDER   = "folder"
    ADDRESS  = "address"
    QUESTION = "question"


class StepStatus(str, Enum):
    PENDING   = "pending"
    RUNNING   = "running"
    DONE      = "done"
    SKIPPED   = "skipped"
    FAILED    = "failed"
    WAITING   = "waiting_approval"


class AIProvider(str, Enum):
    CLAUDE   = "claude"
    OPENAI   = "openai"
    OLLAMA   = "ollama"
    NONE     = "none"


@dataclass
class Finding:
    id:             str            = field(default_factory=lambda: str(uuid.uuid4())[:8])
    tool:           str            = ""
    title:          str            = ""
    severity:       Severity       = Severity.INFO
    domain:         Domain         = Domain.GENERAL
    description:    str            = ""
    file:           str            = ""
    line:           int            = 0
    code_snippet:   str            = ""
    fix_suggestion: str            = ""
    impact:         str            = ""
    references:     list[str]      = field(default_factory=list)
    bug_type:       str            = ""
    raw:            str            = ""
    ai_analysis:    str            = ""
    reproduction:   dict[str, str] = field(default_factory=dict)
    timestamp:      float          = field(default_factory=time.time)

    @property
    def location(self) -> str:
        if self.file and self.line:
            return f"{self.file}:{self.line}"
        return self.file or "unknown"


@dataclass
class PlanStep:
    id:          str        = field(default_factory=lambda: str(uuid.uuid4())[:8])
    title:       str        = ""
    description: str        = ""
    tool:        str        = ""
    command:     str        = ""
    status:      StepStatus = StepStatus.PENDING
    output:      str        = ""
    duration:    float      = 0.0
    findings:    list[Finding] = field(default_factory=list)


@dataclass
class AgentPlan:
    goal:        str            = ""
    steps:       list[PlanStep] = field(default_factory=list)
    approved:    bool           = False
    created_at:  float          = field(default_factory=time.time)


@dataclass
class ChatMessage:
    role:      str   = "user"     # "user" | "agent" | "system" | "tool"
    content:   str   = ""
    tool_name: str   = ""
    timestamp: float = field(default_factory=time.time)


@dataclass
class AIConfig:
    provider:      AIProvider = AIProvider.NONE
    api_key:       str        = ""
    model:         str        = ""
    ollama_url:    str        = "http://localhost:11434"
    ollama_model:  str        = "llama3"
    etherscan_key: str        = ""
    github_token:  str        = ""
    github_repo:   str        = ""

    def is_ready(self) -> bool:
        if self.provider == AIProvider.NONE:
            return False
        if self.provider == AIProvider.OLLAMA:
            return True
        return bool(self.api_key)


@dataclass
class Session:
    id:           str            = field(default_factory=lambda: str(uuid.uuid4())[:12])
    target:       str            = ""
    input_mode:   InputMode      = InputMode.QUESTION
    domain:       Domain         = Domain.GENERAL
    ai_config:    Optional[AIConfig] = None
    plan:         Optional[AgentPlan] = None
    findings:     list[Finding]  = field(default_factory=list)
    messages:     list[ChatMessage] = field(default_factory=list)
    start_time:   float          = field(default_factory=time.time)
    end_time:     float          = 0.0
    active:       bool           = True

    @property
    def duration(self) -> float:
        end = self.end_time or time.time()
        return end - self.start_time

    @property
    def finding_summary(self) -> dict[str, int]:
        counts = {s.value: 0 for s in Severity}
        for f in self.findings:
            counts[f.severity.value] += 1
        return counts

    @property
    def total_bugs(self) -> int:
        return len(self.findings)

    @property
    def tools_ran(self) -> int:
        return len({f.tool for f in self.findings})
