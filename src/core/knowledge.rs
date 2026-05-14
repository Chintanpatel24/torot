pub fn builtin_knowledge_topics() -> Vec<String> {
    vec![
        "attack-surface-mapping".to_string(),
        "subdomain-enumeration".to_string(),
        "web-application-testing".to_string(),
        "api-security".to_string(),
        "secrets-exposure".to_string(),
        "network-recon".to_string(),
        "sandbox-aware-execution".to_string(),
        "cloud-enumeration".to_string(),
        "code-analysis".to_string(),
        "social-engineering".to_string(),
    ]
}

pub fn topic_description(topic: &str) -> &'static str {
    match topic {
        "attack-surface-mapping" => "Map the full attack surface of the target including subdomains, IP ranges, and technologies.",
        "subdomain-enumeration" => "Discover subdomains using passive and active techniques.",
        "web-application-testing" => "Test web applications for common vulnerabilities (XSS, SQLi, SSRF, etc.).",
        "api-security" => "Audit API endpoints for authentication, authorization, and injection flaws.",
        "secrets-exposure" => "Scan for exposed secrets, tokens, and credentials in code and configs.",
        "network-recon" => "Perform network reconnaissance including port scanning and service detection.",
        "sandbox-aware-execution" => "Execute tools in a sandboxed environment with restricted permissions.",
        "cloud-enumeration" => "Enumerate cloud infrastructure including S3 buckets, Azure blobs, and more.",
        "code-analysis" => "Perform static analysis on source code to find vulnerabilities.",
        "social-engineering" => "Gather OSINT for social engineering attack vectors.",
        _ => "General security testing knowledge area.",
    }
}
