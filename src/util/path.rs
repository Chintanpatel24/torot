use std::path::{Path, PathBuf};

pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = home_dir() {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

pub fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
}

pub fn data_dir() -> PathBuf {
    home_dir()
        .map(|h| Path::new(&h).join(".torot"))
        .unwrap_or_else(|| PathBuf::from(".torot"))
}

pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
