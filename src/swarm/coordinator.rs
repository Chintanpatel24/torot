use crate::swarm::circuit_breaker::CircuitBreakerPool;
use crate::swarm::planner::build_execution_plan;
use crate::swarm::{AgentTask, AgentStatus, SwarmConfig, SwarmState};
use std::sync::Arc;

type ExecutorFn = Arc<dyn Fn(&AgentTask) -> tokio::task::JoinHandle<Vec<String>> + Send + Sync>;

pub struct QueenCoordinator {
    swarm: SwarmState,
    config: SwarmConfig,
    breakers: CircuitBreakerPool,
    logger: Option<Box<dyn Fn(&str) + Send>>,
}

impl QueenCoordinator {
    pub fn new(id: &str, config: SwarmConfig) -> Self {
        Self {
            swarm: SwarmState {
                id: id.to_string(),
                tasks: std::collections::HashMap::new(),
                memory: std::collections::HashMap::new(),
                running: false,
            },
            config,
            breakers: CircuitBreakerPool::new(),
            logger: None,
        }
    }

    pub fn with_logger(mut self, logger: Box<dyn Fn(&str) + Send>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn log(&self, msg: &str) {
        if let Some(ref logger) = self.logger {
            logger(msg);
        } else {
            println!("[swarm] {msg}");
        }
    }

    pub fn register_task(&mut self, task: AgentTask) {
        let id = task.id.clone();
        self.swarm.tasks.insert(id.clone(), task);
        self.swarm.memory.insert(format!("task:{id}:status"), "registered".to_string());
        self.log(&format!("Registered task: {id}"));
    }

    pub fn mem_store(&mut self, key: &str, value: &str) {
        self.swarm.memory.insert(key.to_string(), value.to_string());
    }

    pub fn mem_get(&self, key: &str) -> Option<&String> {
        self.swarm.memory.get(key)
    }

    pub fn build_plan(&self) -> Vec<Vec<AgentTask>> {
        let tasks: Vec<AgentTask> = self.swarm.tasks.values().cloned().collect();
        let waves = build_execution_plan(&tasks);
        self.log(&format!("Execution plan: {} wave(s)", waves.len()));
        waves
    }

    pub async fn orchestrate(&mut self, executor: ExecutorFn) {
        self.swarm.running = true;
        self.mem_store("swarm:status", "running");
        self.log("Queen initializing swarm");

        let plan = self.build_plan();

        for wave in plan {
            if !self.swarm.running {
                break;
            }

            let breaker = self.breakers.get("swarm");
            if breaker.is_open() {
                self.log("Circuit breaker OPEN - stopping");
                break;
            }
            let task_count = wave.len();
            let max_par = self.config.max_parallel;

            self.log(&format!("Running wave of {task_count} tasks"));

            let wave_tasks: Vec<AgentTask> = wave;
            let mut handles = Vec::new();

            for task in &wave_tasks {
                if handles.len() >= max_par {
                    if let Some(h) = handles.pop() {
                        let _ = h.await;
                    }
                }
                self.mem_store(&format!("task:{}:status", task.id), "running");
                handles.push(executor(task));
            }
            for h in handles {
                let _ = h.await;
            }

            for task in &wave_tasks {
                self.mem_store(&format!("task:{}:status", task.id), "done");
            }

            self.breakers.get("swarm").record_success();
        }

        self.swarm.running = false;
        self.mem_store("swarm:status", "complete");
        self.log("Orchestration complete");
    }

    pub fn stop(&mut self) {
        self.swarm.running = false;
        self.mem_store("swarm:status", "stopped");
    }

    pub fn get_summary(&self) -> (usize, usize, usize) {
        let tasks: Vec<&AgentTask> = self.swarm.tasks.values().collect();
        let done = tasks.iter().filter(|t| t.status == AgentStatus::Done).count();
        let failed = tasks.iter().filter(|t| t.status == AgentStatus::Failed).count();
        let pending = tasks.iter().filter(|t| matches!(t.status, AgentStatus::Idle | AgentStatus::Running)).count();
        (done, failed, pending)
    }
}
