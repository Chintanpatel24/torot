use crate::swarm::AgentTask;
use std::collections::HashSet;

pub fn build_execution_plan(tasks: &[AgentTask]) -> Vec<Vec<AgentTask>> {
    let mut waves: Vec<Vec<AgentTask>> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut remaining: Vec<&AgentTask> = tasks.iter().collect();

    while !remaining.is_empty() {
        let wave: Vec<AgentTask> = remaining
            .iter()
            .filter(|t| t.depends_on.iter().all(|dep| done.contains(dep)))
            .map(|t| (*t).clone())
            .collect();

        if wave.is_empty() {
            break;
        }

        let mut wave_sorted = wave;
        wave_sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for task in &wave_sorted {
            done.insert(task.id.clone());
        }

        waves.push(wave_sorted);
        remaining.retain(|t| !done.contains(&t.id));
    }

    waves
}

pub fn build_scan_tasks(tools: &[String], _target: &str, _domain: &str) -> Vec<AgentTask> {
    let mut tasks: Vec<AgentTask> = Vec::new();
    let mut id_counter = 0;

    let mut add_task = |tasks: &mut Vec<AgentTask>, name: &str, tool: &str, priority: u8| {
        id_counter += 1;
        let task = AgentTask::new(&format!("task-{id_counter}"), name, tool, priority);
        tasks.push(task);
    };

    if tools.contains(&"semgrep".to_string()) {
        add_task(&mut tasks, "Pattern Analysis", "semgrep", 10);
    }

    let static_tools = [
        "slither", "aderyn", "mythril", "solhint", "clippy", "cargo-audit",
    ];
    for tool in static_tools {
        if tools.contains(&tool.to_string()) {
            add_task(&mut tasks, &format!("Static: {tool}"), tool, 8);
        }
    }

    let dynamic_tools = [
        "echidna", "manticore", "halmos", "nuclei", "nikto", "ffuf", "gobuster", "sqlmap",
    ];
    for tool in dynamic_tools {
        if tools.contains(&tool.to_string()) {
            add_task(&mut tasks, &format!("Dynamic: {tool}"), tool, 6);
        }
    }

    for tool in ["trufflehog", "gitleaks"] {
        if tools.contains(&tool.to_string()) {
            add_task(&mut tasks, &format!("Leak scan: {tool}"), tool, 5);
        }
    }

    tasks
}
