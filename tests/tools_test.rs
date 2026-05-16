use torot_lib::core::tools::{infer_target_kind, host_from_target, url_from_target};

#[test]
fn test_infer_target_kind_url() {
    assert_eq!(infer_target_kind("https://example.com"), "url");
    assert_eq!(infer_target_kind("http://example.com/path"), "url");
}

#[test]
fn test_infer_target_kind_host() {
    assert_eq!(infer_target_kind("example.com"), "host");
    assert_eq!(infer_target_kind("192.168.1.1"), "host");
}

#[test]
fn test_host_from_target() {
    assert_eq!(host_from_target("https://example.com/path"), "example.com");
    assert_eq!(host_from_target("http://example.com"), "example.com");
    assert_eq!(host_from_target("example.com"), "example.com");
}

#[test]
fn test_url_from_target() {
    assert_eq!(url_from_target("https://example.com"), "https://example.com");
    assert_eq!(url_from_target("example.com"), "https://example.com");
    assert_eq!(url_from_target("http://test.com/path"), "http://test.com/path");
}

#[test]
fn test_tool_statuses_empty_config() {
    let config = torot_lib::core::AppConfig::default();
    let statuses = torot_lib::core::tools::tool_statuses(&config);
    assert!(!statuses.is_empty());
    assert!(statuses.iter().any(|t| t.name == "nmap"));
    assert!(statuses.iter().any(|t| t.name == "nuclei"));
}
