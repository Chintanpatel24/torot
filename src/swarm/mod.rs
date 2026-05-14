mod circuit_breaker;
mod coordinator;
mod executor;
mod planner;

pub use circuit_breaker::*;
pub use coordinator::*;
pub use executor::*;
pub use planner::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Idle,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone)]
pub struct AgentTask {
    pub id: String,
    pub name: String,
    pub tool: String,
    pub priority: u8,
    pub status: AgentStatus,
    pub started_at: Option<u64>,
    pub ended_at: Option<u64>,
    pub output: Vec<String>,
    pub error: Option<String>,
    pub depends_on: Vec<String>,
}

impl AgentTask {
    pub fn new(id: &str, name: &str, tool: &str, priority: u8) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            tool: tool.to_string(),
            priority,
            status: AgentStatus::Idle,
            started_at: None,
            ended_at: None,
            output: Vec::new(),
            error: None,
            depends_on: Vec::new(),
        }
    }

    pub fn with_dep(mut self, dep: &str) -> Self {
        self.depends_on.push(dep.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SwarmConfig {
    pub max_parallel: usize,
    pub retry_limit: u32,
    pub timeout_ms: u64,
    pub topology: SwarmTopology,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            retry_limit: 2,
            timeout_ms: 300_000,
            topology: SwarmTopology::Hierarchical,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SwarmTopology {
    Hierarchical,
    Mesh,
    Star,
}

#[derive(Debug, Clone)]
pub struct SwarmState {
    pub id: String,
    pub tasks: HashMap<String, AgentTask>,
    pub memory: HashMap<String, String>,
    pub running: bool,
}
