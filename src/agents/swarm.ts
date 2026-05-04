/**
 * Torot Swarm Agent Engine
 * - Task decomposition (agent-orchestrator-task)
 * - Hierarchical coordination (agent-hierarchical-coordinator)
 * - Circuit breaker resilience (shared/resilience/circuit-breaker)
 * - Memory-backed state (agent-memory-coordinator)
 */

export type AgentStatus = "idle" | "running" | "done" | "failed";

export interface AgentTask {
  id:         string;
  name:       string;
  tool:       string;
  priority:   number;
  status:     AgentStatus;
  startedAt?: number;
  endedAt?:   number;
  output:     string[];
  error?:     string;
  dependsOn:  string[];
}

export interface SwarmConfig {
  maxParallel:  number;
  retryLimit:   number;
  timeoutMs:    number;
  topology:     "hierarchical" | "mesh" | "star";
}

export interface SwarmState {
  id:      string;
  tasks:   Map<string, AgentTask>;
  memory:  Map<string, unknown>;
  running: boolean;
}

// ─── Circuit Breaker (from ruflo resilience patterns) ─────────────────────────

enum CircuitState { CLOSED, OPEN, HALF_OPEN }

class CircuitBreaker {
  private state     = CircuitState.CLOSED;
  private failures  = 0;
  private lastFail  = 0;
  constructor(
    private threshold = 3,
    private resetMs   = 30_000,
  ) {}

  isOpen(): boolean {
    if (this.state === CircuitState.OPEN) {
      if (Date.now() - this.lastFail > this.resetMs) {
        this.state = CircuitState.HALF_OPEN;
        return false;
      }
      return true;
    }
    return false;
  }

  recordSuccess() {
    this.failures = 0;
    this.state = CircuitState.CLOSED;
  }

  recordFailure() {
    this.failures++;
    this.lastFail = Date.now();
    if (this.failures >= this.threshold) {
      this.state = CircuitState.OPEN;
    }
  }
}

// ─── Queen Coordinator (from ruflo queen-coordinator pattern) ─────────────────
