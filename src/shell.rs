use crate::infra::fs;

/// Generate init.sh — ServBay-style: fixed shims PATH + env var loading.
pub fn render_init(binary_path: &str) -> String {
    let home = fs::envswitch_home();
    let shims = home.join("shims");

    format!(
        r##"# envSwitch shell integration
# shims PATH (symlink-based, no eval needed for switching)
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
