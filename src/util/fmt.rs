pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

pub fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - s.len()))
    }
}

pub fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        format!("{count} {word}")
    } else {
        format!("{count} {word}s")
    }
}

pub fn comma_list(items: &[String]) -> String {
    items.join(", ")
}

pub fn bold(s: &str) -> String {
    format!("\x1b[1m{s}\x1b[22m")
}

pub fn red(s: &str) -> String {
    format!("\x1b[31m{s}\x1b[39m")
}

pub fn green(s: &str) -> String {
    format!("\x1b[32m{s}\x1b[39m")
}

pub fn yellow(s: &str) -> String {
    format!("\x1b[33m{s}\x1b[39m")
}

pub fn dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[22m")
}
