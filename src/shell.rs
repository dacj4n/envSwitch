use serde::Serialize;

use crate::infra::fs;

const MARKER_START: &str = "# >>> envswitch initialize >>>";
const MARKER_END: &str = "# <<< envswitch initialize <<<";

#[derive(Debug, Clone, Serialize)]
pub struct InitStatus {
    pub cli_available: bool,
    pub cli_path: String,
    pub shell_initialized: bool,
    pub init_shell: String,
    pub home_dir_exists: bool,
    pub shims_in_path: bool,
}

/// Search for the envswitch CLI binary in PATH and common locations.
/// Equivalent to `which envswitch` — only returns a real CLI binary,
/// never the GUI app bundle.
fn find_cli_path() -> Option<String> {
    // Search PATH for "envswitch"
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = std::path::PathBuf::from(dir).join("envswitch");
            if candidate.exists() && candidate.is_file() {
                return Some(candidate.display().to_string());
            }
        }
    }
    // Common install locations
    for c in &[
        dirs::home_dir()
            .unwrap_or_default()
            .join(".cargo/bin/envswitch"),
        std::path::PathBuf::from("/usr/local/bin/envswitch"),
        std::path::PathBuf::from("/opt/homebrew/bin/envswitch"),
    ] {
        if c.exists() && c.is_file() {
            return Some(c.display().to_string());
        }
    }
    None
}

/// Check whether envswitch CLI is available and whether shell integration
/// has been initialized. Used by the GUI to warn users before cover/uncover.
pub fn check_init_status() -> InitStatus {
    let cli_path = find_cli_path().unwrap_or_default();
    let cli_available = !cli_path.is_empty();

    let home = fs::envswitch_home();
    let home_dir_exists = home.exists();
    let shims_in_path = std::env::var("PATH")
        .unwrap_or_default()
        .contains("envswitch/shims");

    let mut shell_initialized = false;
    let mut init_shell = String::from("none");

    for shell in &["zsh", "bash"] {
        let rc = rc_path(shell);
        if let Ok(content) = std::fs::read_to_string(&rc) {
            if has_init_block(&content) {
                shell_initialized = true;
                if init_shell == "none" {
                    init_shell = shell.to_string();
                } else {
                    init_shell = format!("{},{}", init_shell, shell);
                }
            }
        }
    }

    InitStatus {
        cli_available,
        cli_path,
        shell_initialized,
        init_shell,
        home_dir_exists,
        shims_in_path,
    }
}

fn rc_path(shell: &str) -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    match shell {
        "bash" => home.join(".bashrc"),
        _ => home.join(".zshrc"),
    }
}

/// Generate the shell integration block (for insertion into .zshrc / .bashrc).
pub fn render_init_block(binary_path: &str) -> String {
    let home = fs::envswitch_home();
    let shims = home.join("shims");

    format!(
        r##"{marker_start}
export _ENVSWITCH_BIN="{binary_path}"
export _ENVSWITCH_HOME="{home}"

# Add shims to PATH (once per shell — idempotent guard)
if [ -z "$_ENVSWITCH_LOADED" ]; then
    export PATH="{shims}:$PATH"
    _ENVSWITCH_LOADED=1
fi

# Refresh env vars from state on each prompt
_envswitch_refresh() {{
    local env_sh="${{_ENVSWITCH_HOME}}/state/env.sh"
    [ -f "$env_sh" ] && source "$env_sh"
}}

if [ -n "$ZSH_VERSION" ]; then
    precmd() {{ _envswitch_refresh; hash -r 2>/dev/null; }}
elif [ -n "$BASH_VERSION" ]; then
    PROMPT_COMMAND="_envswitch_refresh; ${{PROMPT_COMMAND:+$PROMPT_COMMAND;}}hash -r 2>/dev/null"
fi

# Load env on shell startup
_envswitch_refresh

# Auto cd-hook: detect .envswitchrc when changing directories
__envswitch_cd_hook() {{
    local hook_file="${{_ENVSWITCH_HOME}}/config/cd-hook"
    if [ -f "$hook_file" ] && [ "$(cat "$hook_file")" = "on" ]; then
        cd() {{
            builtin cd "$@" || return
            if [ -f ".envswitchrc" ]; then
                "$_ENVSWITCH_BIN" auto
            fi
        }}
    fi
}}
__envswitch_cd_hook
{marker_end}
"##,
        marker_start = MARKER_START,
        binary_path = binary_path,
        home = home.display(),
        shims = shims.display(),
        marker_end = MARKER_END,
    )
}

/// Check if a shell rc file already contains the envswitch integration block.
pub fn has_init_block(rc_content: &str) -> bool {
    rc_content.contains(MARKER_START) && rc_content.contains(MARKER_END)
}

/// Remove the envswitch init block from a shell rc file's content.
/// Returns the cleaned content, or unchanged if no block was found.
pub fn remove_init_block(rc_content: &str) -> String {
    if let Some(start) = rc_content.find(MARKER_START) {
        if let Some(end) = rc_content.find(MARKER_END) {
            let end_pos = end + MARKER_END.len();
            let before = &rc_content[..start];
            let after = &rc_content[end_pos..];
            // Remove the block and clean up trailing/leading newlines
            let before = before.trim_end();
            let after = after.trim_start();
            if before.is_empty() {
                return after.to_string();
            }
            return format!("{}\n{}", before, after);
        }
    }
    rc_content.to_string()
}
