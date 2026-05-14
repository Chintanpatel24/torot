#[derive(Debug, Clone)]
pub enum SandboxProfile {
    Strong,
    Moderate,
    Off,
}

impl SandboxProfile {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "moderate" => SandboxProfile::Moderate,
            "off" => SandboxProfile::Off,
            _ => SandboxProfile::Strong,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SandboxProfile::Strong => "strong",
            SandboxProfile::Moderate => "moderate",
            SandboxProfile::Off => "off",
        }
    }

    pub fn env_clear(&self) -> bool {
        matches!(self, SandboxProfile::Strong | SandboxProfile::Moderate)
    }

    pub fn restrict_network(&self) -> bool {
        matches!(self, SandboxProfile::Strong)
    }
}
