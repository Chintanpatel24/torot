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

