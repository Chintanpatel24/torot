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

export class QueenCoordinator {
  private swarm:           SwarmState;
  private circuitBreakers: Map<string, CircuitBreaker> = new Map();
  private onLog:           (msg: string) => void;

  constructor(
    swarmId: string,
    private config: SwarmConfig,
    onLog: (msg: string) => void = console.log,
  ) {
    this.onLog = onLog;
    this.swarm = {
      id:      swarmId,
      tasks:   new Map(),
      memory:  new Map(),
      running: false,
    };
  }

  // Memory operations (from ruflo memory-coordinator patterns)
  memStore(key: string, value: unknown): void {
    this.swarm.memory.set(key, value);
  }

  memGet<T>(key: string): T | undefined {
    return this.swarm.memory.get(key) as T | undefined;
  }

  memSearch(prefix: string): [string, unknown][] {
    const results: [string, unknown][] = [];
    this.swarm.memory.forEach((v, k) => {
      if (k.startsWith(prefix)) results.push([k, v]);
    });
    return results;
  }

  // Task registration
  registerTask(task: Omit<AgentTask, "status" | "output">): void {
    this.swarm.tasks.set(task.id, {
      ...task,
      status: "idle",
      output: [],
    });
    this.memStore(`task:${task.id}:status`, "registered");
    this.onLog(`[swarm] Registered task: ${task.name} (${task.tool})`);
  }

  // Build execution plan — respects dependencies (from agent-orchestrator-task)

 buildExecutionPlan(): AgentTask[][] {
    const tasks   = Array.from(this.swarm.tasks.values());
    const waves: AgentTask[][] = [];
    const done    = new Set<string>();

    let remaining = [...tasks];
    while (remaining.length > 0) {
      const wave = remaining.filter((t) =>
        t.dependsOn.every((dep) => done.has(dep))
      );
      if (wave.length === 0) break; // cycle guard
      waves.push(wave.sort((a, b) => b.priority - a.priority));
      wave.forEach((t) => done.add(t.id));
      remaining = remaining.filter((t) => !done.has(t.id));
    }

    this.onLog(`[swarm] Execution plan: ${waves.length} wave(s)`);
    this.memStore("swarm:plan", waves.map((w) => w.map((t) => t.name)));
    return waves;
  }

  // Run a wave of tasks in parallel (respects maxParallel)
  async runWave(
    wave:      AgentTask[],
    executor:  (task: AgentTask) => Promise<string[]>,
    onUpdate:  (task: AgentTask) => void,
  ): Promise<void> {
    const chunks: AgentTask[][] = [];
    for (let i = 0; i < wave.length; i += this.config.maxParallel) {
      chunks.push(wave.slice(i, i + this.config.maxParallel));
    }

    for (const chunk of chunks) {
      await Promise.all(chunk.map(async (task) => {
        const cb = this.getCircuitBreaker(task.tool);
        if (cb.isOpen()) {
          task.status = "failed";
          task.error  = `Circuit breaker OPEN for ${task.tool}`;
          this.onLog(`[swarm] Circuit breaker open — skipping ${task.tool}`);
          onUpdate(task);
          return;
        }

        task.status    = "running";
        task.startedAt = Date.now();
        this.memStore(`task:${task.id}:status`, "running");
        onUpdate(task);
        this.onLog(`[swarm] Running: ${task.name}`);

        try {
          const output = await Promise.race([
            executor(task),
            new Promise<never>((_, reject) =>
              setTimeout(() => reject(new Error("timeout")), this.config.timeoutMs)
            ),
          ]);
          task.output  = output;
          task.status  = "done";
          task.endedAt = Date.now();
          cb.recordSuccess();
          this.memStore(`task:${task.id}:status`,  "done");
          this.memStore(`task:${task.id}:output`,  output.slice(-10));
          this.onLog(`[swarm] Done: ${task.name} (${output.length} lines)`);
        } catch (err) {
          task.status  = "failed";
          task.error   = String(err);
          task.endedAt = Date.now();
          cb.recordFailure();
          this.memStore(`task:${task.id}:status`, "failed");
          this.onLog(`[swarm] Failed: ${task.name} — ${task.error}`);
        }

        onUpdate(task);
      }));
    }
  }

  // Full orchestrated scan
  async orchestrate(
    executor:  (task: AgentTask) => Promise<string[]>,
    onUpdate:  (task: AgentTask) => void,
  ): Promise<void> {
    this.swarm.running = true;
    this.memStore("swarm:status", "running");
    this.onLog(`[swarm] Queen initializing swarm ${this.swarm.id}`);

    const plan = this.buildExecutionPlan();
    for (const wave of plan) {
      if (!this.swarm.running) break;
      await this.runWave(wave, executor, onUpdate);
    }

    this.swarm.running = false;
    this.memStore("swarm:status", "complete");
    this.onLog(`[swarm] Orchestration complete`);
  }

  stop(): void {
    this.swarm.running = false;
    this.memStore("swarm:status", "stopped");
  }

  getSummary(): { done: number; failed: number; pending: number } {
    const tasks = Array.from(this.swarm.tasks.values());
    return {
      done:    tasks.filter((t) => t.status === "done").length,
      failed:  tasks.filter((t) => t.status === "failed").length,
      pending: tasks.filter((t) => t.status === "idle" || t.status === "running").length,
    };
  }

  private getCircuitBreaker(tool: string): CircuitBreaker {
    if (!this.circuitBreakers.has(tool)) {
      this.circuitBreakers.set(tool, new CircuitBreaker());
    }
    return this.circuitBreakers.get(tool)!;
  }
}

// ─── Plan builder from ruflo agent-goal-planner patterns ─────────────────────

export function buildScanTasks(
  tools:    string[],
  target:   string,
  domain:   string,
): Omit<AgentTask, "status" | "output">[] {
  const tasks: Omit<AgentTask, "status" | "output">[] = [];

  // Recon phase (priority 10)
  if (tools.includes("semgrep")) {
    tasks.push({ id: "recon-semgrep", name: "Pattern Analysis", tool: "semgrep", priority: 10, dependsOn: [] });
  }
  if (tools.includes("strings")) {
    tasks.push({ id: "recon-strings", name: "String Extraction", tool: "strings", priority: 9, dependsOn: [] });
  }

  // Static analysis (priority 8, can run in parallel)
  const staticTools = ["slither", "aderyn", "mythril", "solhint", "wake", "solc", "clippy", "cargo-audit"];
  staticTools.filter((t) => tools.includes(t)).forEach((tool, i) => {
    tasks.push({ id: `static-${tool}`, name: `Static: ${tool}`, tool, priority: 8 - i, dependsOn: [] });
  });

  // Dynamic / fuzzing (priority 6, after static)
  const dynamicTools = ["echidna", "manticore", "halmos", "nuclei", "nikto", "ffuf", "gobuster", "sqlmap", "dalfox"];
  const staticIds = tasks.filter((t) => t.id.startsWith("static")).map((t) => t.id);
  dynamicTools.filter((t) => tools.includes(t)).forEach((tool, i) => {
    tasks.push({ id: `dynamic-${tool}`, name: `Dynamic: ${tool}`, tool, priority: 6 - i, dependsOn: [] });
  });

  // Secret/leak scanning (priority 5)
  ["trufflehog", "gitleaks"].filter((t) => tools.includes(t)).forEach((tool) => {
    tasks.push({ id: `leak-${tool}`, name: `Leak scan: ${tool}`, tool, priority: 5, dependsOn: [] });
  });

  // Binary analysis (priority 4)
  ["radare2", "binwalk", "checksec", "objdump"].filter((t) => tools.includes(t)).forEach((tool, i) => {
    tasks.push({ id: `binary-${tool}`, name: `Binary: ${tool}`, tool, priority: 4 - i, dependsOn: [] });
  });

  return tasks;
}
