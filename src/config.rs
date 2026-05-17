//! Configuration file management for envSwitch.
//! Stores settings in ~/.envswitch/config.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvSwitchConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

fn config_path() -> PathBuf {
    crate::infra::fs::envswitch_home().join("config.json")
}

pub fn load_config() -> EnvSwitchConfig {
    let path = config_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        EnvSwitchConfig::default()
    }
}

#[allow(dead_code)]
fn save_config(config: &EnvSwitchConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(config).unwrap_or_default(),
    );
}

/// Get the configured proxy URL, if any.
pub fn get_proxy() -> Option<String> {
    load_config().proxy.filter(|p| !p.is_empty())
}

/// Set the proxy URL. Pass empty string to clear.
#[allow(dead_code)]
pub fn set_proxy(proxy: &str) {
    let mut config = load_config();
    if proxy.is_empty() {
        config.proxy = None;
    } else {
        config.proxy = Some(proxy.to_string());
    }
    save_config(&config);
}

/// Apply proxy environment variables to a Command if proxy is configured.
pub fn apply_proxy(cmd: &mut std::process::Command) {
    if let Some(proxy) = get_proxy() {
        cmd.env("HTTP_PROXY", &proxy)
            .env("HTTPS_PROXY", &proxy)
            .env("http_proxy", &proxy)
            .env("https_proxy", &proxy)
            .env("ALL_PROXY", &proxy)
            .env("all_proxy", &proxy);
    }
}
