use crate::infra::fs;

const MARKER_START: &str = "# >>> envswitch initialize >>>";
const MARKER_END: &str = "# <<< envswitch initialize <<<";

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

# Shell function: eval only env vars for cover/uncover/auto
envswitch() {{
    case "$1" in
        cover|uncover|auto)
            eval "$("$_ENVSWITCH_BIN" "$@")"
            ;;
        *)
            "$_ENVSWITCH_BIN" "$@"
            ;;
    esac
}}

# Auto-clear hash table on each prompt (shims may have changed)
precmd() {{ hash -r 2>/dev/null; }} 2>/dev/null
precmd_functions+=(precmd) 2>/dev/null

# Auto cd-hook: detect .envswitchrc when changing directories
__envswitch_cd_hook() {{
    local hook_file="${{_ENVSWITCH_HOME}}/config/cd-hook"
    if [ -f "$hook_file" ] && [ "$(cat "$hook_file")" = "on" ]; then
        cd() {{
            builtin cd "$@" || return
            if [ -f ".envswitchrc" ]; then
                eval "$("$_ENVSWITCH_BIN" auto 2>/dev/null)"
            fi
        }}
    fi
}}
__envswitch_cd_hook

# Load env vars from global covers on shell startup
__envswitch_load_global() {{
    eval "$("$_ENVSWITCH_BIN" load-globals 2>/dev/null)"
}}
__envswitch_load_global
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
