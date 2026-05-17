//! Global operation log — records all user actions with timestamps and levels.
//! Stored in ~/.envswitch/logs/operations.log

use crate::infra::fs::envswitch_home;
use chrono::Local;

#[allow(dead_code)]
pub enum OpLevel {
    Ok,
    Info,
    Warn,
    Error,
}

#[allow(dead_code)]
impl OpLevel {
    fn tag(&self) -> &str {
        match self {
            OpLevel::Ok => "OK",
            OpLevel::Info => "INFO",
            OpLevel::Warn => "WARN",
            OpLevel::Error => "ERR",
        }
    }
}

/// Write an operation log entry.
#[allow(dead_code)]
pub fn log_op(level: OpLevel, msg: &str) {
    let dir = envswitch_home().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("operations.log");
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("{}  {}  {}\n", now, level.tag(), msg);
    // Append
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Read the last `max_lines` from the operation log.
#[allow(dead_code)]
pub fn read_ops(max_lines: usize) -> Vec<String> {
    let path = envswitch_home().join("logs").join("operations.log");
    if !path.exists() {
        return vec![];
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let all: Vec<&str> = content.lines().collect();
    let start = all.len().saturating_sub(max_lines);
    all[start..].iter().map(|s| s.to_string()).collect()
}
