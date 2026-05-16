use crate::infra::fs;

/// Generate init.sh — ServBay-style: fixed shims PATH + env var loading.
pub fn render_init(binary_path: &str) -> String {
    let home = fs::envswitch_home();
    let shims = home.join("shims");

    format!(
        r##"# envSwitch shell integration
# Idempotent guard — only source once
if [ -n "$_ENVSWITCH_LOADED" ]; then return; fi
_ENVSWITCH_LOADED=1

export PATH="{}:$PATH"

_ENVSWITCH_BIN="{}"
_ENVSWITCH_HOME="{}"

# Shell function: eval only env vars (PATH changes via shims symlinks)
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
    local sf="${{_ENVSWITCH_HOME}}/state/global.json"
    if [ -f "$sf" ] && command -v python3 >/dev/null 2>&1; then
        eval "$(python3 -c "
import json
with open('$sf') as f:
    data = json.load(f)
for c in data.get('covers', []):
    mod = c.get('module_name','')
    ver = c.get('version','')
    print(f'envswitch cover {{mod}} {{ver}} --global 2>/dev/null')
" 2>/dev/null)"
    fi
}}
__envswitch_load_global
"##,
        shims.display(),
        binary_path,
        home.display(),
    )
}
