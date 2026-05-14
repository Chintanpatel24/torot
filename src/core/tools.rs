use crate::core::types::{ToolProfile, ToolStatus, WizardStep, AppConfig};
use std::path::Path;
use std::process::Command as StdCommand;

#[derive(Debug, Clone)]
pub struct ToolRuntime {
    pub installed: bool,
    pub binary: Option<String>,
    pub version: Option<String>,
}

pub fn detect_tool(profile: &ToolProfile) -> ToolRuntime {
    let candidate = profile
        .path_override
        .clone()
        .filter(|p| Path::new(p).exists())
        .or_else(|| {
            profile.binary_names.iter().find_map(|bin| {
                which::which(bin).ok().map(|p| p.to_string_lossy().to_string())
            })
        });

    let version = candidate
        .as_ref()
        .and_then(|binary| detect_version(binary, &profile.version_args));

    ToolRuntime {
        installed: candidate.is_some(),
        binary: candidate,
        version,
    }
}

fn detect_version(binary: &str, args: &[String]) -> Option<String> {
    if args.is_empty() {
        return None;
    }
    let output = StdCommand::new(binary).args(args).output().ok()?;
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };
    text.lines().next().map(|line| line.trim().to_string())
}

pub fn tool_statuses(config: &AppConfig) -> Vec<ToolStatus> {
    config
        .tools
        .iter()
        .map(|profile| {
            let runtime = detect_tool(profile);
            ToolStatus {
                name: profile.name.clone(),
                installed: runtime.installed,
                binary: runtime.binary.unwrap_or_default(),
                version: runtime.version.unwrap_or_default(),
                domain: profile.domain.clone(),
                description: profile.description.clone(),
                install_hint: profile.install_hint.clone(),
                output_format: profile.output_format.clone(),
                source: profile.source.clone(),
                auto_detect: profile.auto_detect,
                enabled: profile.enabled,
                capabilities: profile.capabilities.clone(),
                knowledge: profile.knowledge.clone(),
                wizard_steps: wizard_steps(profile, runtime.installed),
            }
        })
        .collect()
}

pub fn wizard_steps(profile: &ToolProfile, installed: bool) -> Vec<WizardStep> {
    if installed {
        vec![
            WizardStep { order: 1, title: "Detected".to_string(), detail: "Torot found this tool on your system.".to_string() },
            WizardStep { order: 2, title: "Tune Arguments".to_string(), detail: "Adjust command arguments for your use case.".to_string() },
        ]
    } else {
        vec![
            WizardStep { order: 1, title: "Install Or Locate".to_string(), detail: if profile.install_hint.trim().is_empty() { "Install the tool or paste the binary path.".to_string() } else { format!("Hint: {}", profile.install_hint) } },
            WizardStep { order: 2, title: "Set Binary Path".to_string(), detail: "Add the exact path in Torot tool settings if auto-detection fails.".to_string() },
            WizardStep { order: 3, title: "Choose Arguments".to_string(), detail: "Keep placeholders like {{target}} for Torot to fill.".to_string() },
        ]
    }
}

pub fn get_tools(state: &crate::core::state::AppState) -> Result<Vec<ToolStatus>, String> {
    let config = crate::core::config::load_config(state).map_err(|e| e.to_string())?;
    Ok(tool_statuses(&config))
}

pub fn infer_target_kind(target: &str) -> &'static str {
    let path = Path::new(target);
    if target.starts_with("http://") || target.starts_with("https://") {
        "url"
    } else if path.is_dir() {
        "directory"
    } else if path.is_file() {
        "file"
    } else {
        "host"
    }
}

pub fn host_from_target(target: &str) -> String {
    let raw = target
        .strip_prefix("http://")
        .or_else(|| target.strip_prefix("https://"))
        .unwrap_or(target);
    raw.split('/').next().unwrap_or(raw).to_string()
}

pub fn url_from_target(target: &str) -> String {
    if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else {
        format!("https://{}", target.trim_end_matches('/'))
    }
}

pub fn render_args(profile: &ToolProfile, target: &str, report_file: &Path) -> Option<Vec<String>> {
    let target_kind = infer_target_kind(target);
    if !profile.input_kinds.is_empty() && !profile.input_kinds.iter().any(|k| k == target_kind) {
        return None;
    }

    let host = host_from_target(target);
    let url = url_from_target(target);
    let workspace = if Path::new(target).is_dir() {
        target.to_string()
    } else {
        ".".to_string()
    };
    let report_path = report_file.to_string_lossy().to_string();

    let args = profile
        .args
        .iter()
        .map(|arg| {
            arg.replace("{{target}}", target)
                .replace("{{target_host}}", &host)
                .replace("{{target_url}}", &url)
                .replace("{{workspace}}", &workspace)
                .replace("{{report_file}}", &report_path)
        })
        .collect();
    Some(args)
}

pub fn suggest_tools(config: &AppConfig, target: &str) -> Vec<String> {
    let kind = infer_target_kind(target);
    config
        .tools
        .iter()
        .filter(|tool| tool.enabled)
        .filter(|tool| tool.input_kinds.is_empty() || tool.input_kinds.iter().any(|k| k == kind))
        .filter(|tool| detect_tool(tool).installed)
        .take(4)
        .map(|tool| tool.name.clone())
        .collect()
}
