use std::path::Path;

#[test]
fn test_default_config_creates_valid_json() {
    let dir = std::env::temp_dir().join("torot-test-config");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.json");

    // This simulates what the app does
    let config = torot_lib::core::AppConfig::default();
    let json = serde_json::to_string_pretty(&config).unwrap();
    std::fs::write(&path, &json).unwrap();

    // Read it back
    let raw = std::fs::read_to_string(&path).unwrap();
    let parsed: torot_lib::core::AppConfig = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed.version, "4.0.0");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_default_report_template_has_placeholders() {
    let template = torot_lib::core::default_report_template();
    assert!(template.contains("{{session_id}}"));
    assert!(template.contains("{{target}}"));
    assert!(template.contains("{{findings_total}}"));
    assert!(template.contains("{{critical_count}}"));
    assert!(template.contains("{{summary}}"));
}

#[test]
fn test_report_placeholders() {
    let placeholders = torot_lib::core::report_placeholders();
    assert!(placeholders.contains(&"{{session_id}}".to_string()));
    assert!(placeholders.contains(&"{{findings_table}}".to_string()));
    assert!(placeholders.contains(&"{{tool_overview}}".to_string()));
}
