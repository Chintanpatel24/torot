use crate::core::event::{AppEvent, EventBus};
use crate::core::state::AppState;
use crate::core::tools::detect_tool;
use crate::core::types::{ToolProfile, AppConfig};
use crate::swarm::AgentTask;
use std::collections::HashMap;
use std::sync::Arc;

pub type TaskExecutor = Arc<dyn Fn(&AgentTask) -> tokio::task::JoinHandle<Vec<String>> + Send + Sync>;

pub fn create_default_executor(
    config: AppConfig,
    state: Arc<AppState>,
    bus: EventBus,
) -> TaskExecutor {
    let tools_by_name: Arc<HashMap<String, ToolProfile>> = Arc::new(
        config.tools.iter().cloned().map(|t| (t.name.clone(), t)).collect(),
    );

    Arc::new(move |task: &AgentTask| {
        let tools = Arc::clone(&tools_by_name);
        let state = Arc::clone(&state);
        let bus = bus.clone();
        let task_id = task.id.clone();
        let tool_name = task.tool.clone();

        tokio::spawn(async move {
            let profile = match tools.get(&tool_name) {
                Some(p) => p.clone(),
                None => return vec![format!("[swarm] Unknown tool: {tool_name}")],
            };

            let runtime = detect_tool(&profile);
            let Some(_binary) = runtime.binary else {
                return vec![format!("[swarm] {tool_name} not installed")];
            };

            let session_id = format!("swarm-{task_id}");
            let target = "placeholder".to_string();

            let output = run_tool_simple(&session_id, &target, &profile, 300, bus, state).await;
            output
        })
    })
}

pub async fn run_tool_simple(
    session_id: &str,
    target: &str,
    profile: &ToolProfile,
    max_runtime_seconds: u64,
    bus: EventBus,
    state: Arc<AppState>,
) -> Vec<String> {
    use crate::core::tools::render_args;
    let runtime = detect_tool(profile);
    let Some(binary) = runtime.binary else {
        bus.emit(AppEvent::Line {
            tool: profile.name.clone(),
            line: format!("{} not installed. {}", profile.name, profile.install_hint),
            kind: "system".to_string(),
            severity: Some("MEDIUM".to_string()),
        });
        return Vec::new();
    };

    let report_file = state.reports_dir.join(format!("{session_id}-{}.out", profile.name));
    let args = render_args(profile, target, &report_file);
    let Some(args) = args else {
        return Vec::new();
    };

    let mut command = tokio::process::Command::new(&binary);
    command.args(&args);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let Ok(mut child) = command.spawn() else {
        return Vec::new();
    };

    let mut output = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                output.push(line.clone());
                bus.emit(AppEvent::Line {
                    tool: profile.name.clone(),
                    line,
                    kind: "output".to_string(),
                    severity: None,
                });
            }
        }
    }

    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(max_runtime_seconds),
        child.wait(),
    )
    .await;
    output
}
