use torot_lib::core::report::render_report;
use torot_lib::core::types::{Finding, Session};

fn make_session() -> Session {
    Session::new("https://example.com", "single")
}

fn make_findings() -> Vec<Finding> {
    let mut f1 = Finding::new("s1", "nuclei", "[nuclei] XSS detected", "HIGH");
    f1.description = "XSS in search parameter".to_string();
    f1.file = "https://example.com/search".to_string();

    let mut f2 = Finding::new("s1", "nmap", "[nmap] open port 80", "MEDIUM");

    vec![f1, f2]
}

#[test]
fn test_render_report() {
    let session = make_session();
    let findings = make_findings();
    let template = "# Report\n- Target: {{target}}\n- Findings: {{findings_total}}\n\n{{findings_table}}";

    let report = render_report(template, &session, &findings);
    assert!(report.contains("https://example.com"));
    assert!(report.contains("2"));
    assert!(report.contains("| Severity |"));
    assert!(report.contains("nuclei"));
    assert!(report.contains("nmap"));
}

#[test]
fn test_render_report_empty() {
    let session = make_session();
    let template = "{{summary}}";
    let report = render_report(template, &session, &[]);
    assert!(report.contains("No findings"));
}

#[test]
fn test_render_report_full_template() {
    let session = make_session();
    let findings = make_findings();
    let template = torot_lib::core::default_report_template();

    let report = render_report(&template, &session, &findings);
    assert!(report.contains("Torot v4 Report"));
    assert!(report.contains("Executive Summary"));
    assert!(report.contains("Tool Coverage"));
    assert!(report.contains("Findings"));
}
