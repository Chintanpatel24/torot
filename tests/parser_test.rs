use torot_lib::core::types::{Finding, ToolProfile};
use torot_lib::core::parser::{parse_output, summarize_findings, severity_from_text, severity_rank};

fn make_profile(name: &str) -> ToolProfile {
    ToolProfile {
        name: name.to_string(),
        domain: "webapp".to_string(),
        description: String::new(),
        binary_names: vec![],
        path_override: None,
        args: vec![],
        version_args: vec![],
        install_hint: String::new(),
        output_format: "text".to_string(),
        input_kinds: vec![],
        source: "builtin".to_string(),
        auto_detect: true,
        enabled: true,
        timeout_seconds: 300,
        capabilities: vec![],
        knowledge: vec![],
    }
}

#[test]
fn test_parse_nuclei_json() {
    let sid = "test-session";
    let profile = make_profile("nuclei");
    let output = r#"{"info":{"name":"test-xss","severity":"high","description":"XSS detected"},"matched-at":"https://example.com/xss"}"#;

    let findings = parse_output(sid, &profile, output);
    assert!(!findings.is_empty());
    assert_eq!(findings[0].severity, "HIGH");
    assert!(findings[0].title.contains("test-xss"));
}

#[test]
fn test_parse_semgrep_json() {
    let sid = "test-session";
    let profile = make_profile("semgrep");
    let output = r#"{"results":[{"check_id":"test.rule","path":"src/main.rs","start":{"line":42},"extra":{"severity":"ERROR","message":"Unsafe code"}}]}"#;

    let findings = parse_output(sid, &profile, output);
    assert!(!findings.is_empty());
    assert_eq!(findings[0].severity, "HIGH");
    assert_eq!(findings[0].line, 42);
}

#[test]
fn test_parse_gitleaks_json() {
    let sid = "test-session";
    let profile = make_profile("gitleaks");
    let output = r#"[{"Description":"AWS Secret Key","File":".env","StartLine":1}]"#;

    let findings = parse_output(sid, &profile, output);
    assert!(!findings.is_empty());
    assert_eq!(findings[0].severity, "HIGH");
}

#[test]
fn test_parse_text_findings() {
    let sid = "test-session";
    let profile = make_profile("nmap");
    let output = "22/tcp open ssh\n80/tcp open http\n[critical] Remote code execution in Apache\n";

    let findings = parse_output(sid, &profile, output);
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.severity == "CRITICAL"));
}

#[test]
fn test_empty_output() {
    let sid = "test-session";
    let profile = make_profile("nmap");
    let findings = parse_output(sid, &profile, "");
    assert!(findings.is_empty());
}

#[test]
fn test_severity_from_text() {
    assert_eq!(severity_from_text("critical vulnerability"), "CRITICAL");
    assert_eq!(severity_from_text(" HIGH: remote code execution"), "HIGH");
    assert_eq!(severity_from_text("[error] something broke"), "HIGH");
    assert_eq!(severity_from_text("RCE detected"), "HIGH");
    assert_eq!(severity_from_text("warning detected"), "MEDIUM");
    assert_eq!(severity_from_text("low risk"), "LOW");
    assert_eq!(severity_from_text("info only"), "INFO");
}

#[test]
fn test_severity_rank() {
    assert_eq!(severity_rank("CRITICAL"), 0);
    assert_eq!(severity_rank("HIGH"), 1);
    assert_eq!(severity_rank("MEDIUM"), 2);
    assert_eq!(severity_rank("LOW"), 3);
    assert_eq!(severity_rank("INFO"), 4);
    assert_eq!(severity_rank("UNKNOWN"), 4);
}

#[test]
fn test_summarize_findings() {
    let mut findings = vec![];
    for _ in 0..3 {
        let mut f = Finding::new("s1", "test", "finding", "CRITICAL");
        f.severity = "CRITICAL".to_string();
        findings.push(f);
    }
    for _ in 0..2 {
        let mut f = Finding::new("s1", "test", "finding", "HIGH");
        f.severity = "HIGH".to_string();
        findings.push(f);
    }

    let summary = summarize_findings(&findings);
    assert!(summary.contains("5 finding(s)"));
    assert!(summary.contains("3 critical"));
    assert!(summary.contains("2 high"));
}

#[test]
fn test_summarize_empty() {
    let summary = summarize_findings(&[]);
    assert!(summary.contains("No findings"));
}
