use crate::domain::*;
use crate::infra::fs;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;

/// Add a module version to the cover stack, then rebuild and output the full environment.
pub fn cover(module_name: &str, version: &str, scope: CoverScope) -> Result<String, String> {
    let module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    let install_path = fs::envswitch_home().join("envs").join(module_name).join(version);
    if !install_path.exists() {
        return Err(format!("{} {} is not installed. Run 'envswitch install {} {}' first.",
            module_name, version, module_name, version));
    }

    // Check if same version already at top
    let mut stack = load_stack();
    let already_covered = stack.iter().any(|c| c.module_name == module_name && c.version == version);
    let is_top = stack.last().map_or(false, |c| c.module_name == module_name && c.version == version);
    if is_top {
        eprintln!("{} {} is already the current version.", module_name, version);
        return Ok(render_env()); // still output full env for eval
    }

    // Remove existing entry for this module (replace)
    stack.retain(|c| c.module_name != module_name);

    // Push to end of stack (last = highest PATH priority)
    let data_path = fs::envswitch_home().join("data").join(module_name);
    let install_path = fs::envswitch_home().join("envs").join(module_name).join(version);

    // Compute bin paths
    let mut bin_paths: Vec<PathBuf> = module.path_entries.iter()
        .map(|pe| install_path.join(pe))
        .filter(|p| p.exists())
        .collect();

    if module_name == "jdk" {
        let home = install_path.join("Contents").join("Home");
        if home.exists() {
            bin_paths = vec![home.join("bin")];
        }
    }

    let mut env_vars = HashMap::new();
    for (key, template) in &module.env_vars {
        let value = fs::resolve_template(template, &install_path, &data_path);
        env_vars.insert(key.clone(), value);
    }

    let entry = StackEntry {
        module_name: module_name.to_string(),
        version: version.to_string(),
        scope,
        bin_paths,
        env_vars,
        applied_at: Utc::now(),
    };

    stack.push(entry);
    save_stack(&stack);

    // Rebuild and return full environment script
    Ok(render_full_env(&stack))
}

/// Remove a module from the stack, then rebuild and output the full environment.
pub fn uncover(module_name: &str) -> Result<String, String> {
    let mut stack = load_stack();
    if !stack.iter().any(|c| c.module_name == module_name) {
        eprintln!("{} is not currently covered.", module_name);
        return Ok(String::new());
    }
    stack.retain(|c| c.module_name != module_name);
    save_stack(&stack);
    Ok(render_full_env(&stack))
}

/// Return the full env script for the current stack without modifying it.
pub fn render_env() -> String {
    let stack = load_stack();
    render_full_env(&stack)
}

/// Uncover all modules.
pub fn uncover_all() -> Result<String, String> {
    let stack: Vec<StackEntry> = vec![];
    save_stack(&stack);
    Ok(render_full_env(&stack))
}

/// Rebuild the complete environment from the current stack.
/// Output format: first clear all managed vars, then set from stack, then rebuild PATH.
fn render_full_env(stack: &[StackEntry]) -> String {
    let mut script = String::new();

    // Save original PATH if not already saved
    script.push_str(
        r#": "${ENVSWITCH_SAVED_PATH:=$PATH}"
"#,
    );

    // Collect all managed env var names
    let all_modules = crate::module_repo::builtin_modules();
    let mut all_var_names: Vec<String> = Vec::new();
    for m in &all_modules {
        for (key, _) in &m.env_vars {
            if !all_var_names.contains(key) {
                all_var_names.push(key.clone());
            }
        }
    }

    // Unset all managed vars first (clean slate)
    if !all_var_names.is_empty() {
        script.push_str(&format!("unset {}\n", all_var_names.join(" ")));
    }

    // Set env vars from stack (last entry wins for same key)
    for entry in stack {
        for (key, value) in &entry.env_vars {
            script.push_str(&format!("export {}={}\n", key, value));
        }
    }

    // Rebuild PATH from stack in order (first covered = first in PATH = lowest priority)
    let bin_paths: Vec<String> = stack
        .iter()
        .rev() // reverse: last covered = highest priority
        .flat_map(|e| e.bin_paths.iter().map(|p| p.to_string_lossy().to_string()))
        .collect();

    if bin_paths.is_empty() {
        // No covers: restore original PATH
        script.push_str("export PATH=\"$ENVSWITCH_SAVED_PATH\"\n");
    } else {
        script.push_str(&format!(
            "export PATH=\"{}:$ENVSWITCH_SAVED_PATH\"\n",
            bin_paths.join(":")
        ));
    }

    script
}

/// Get current status.
pub fn get_status() -> Vec<ActiveCover> {
    load_stack()
        .into_iter()
        .map(|e| ActiveCover {
            module_name: e.module_name,
            version: e.version,
            scope: e.scope,
            injected_paths: e.bin_paths,
            injected_envs: e.env_vars.keys().cloned().collect(),
            applied_at: e.applied_at,
        })
        .collect()
}

// ── Stack persistence ─────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StackEntry {
    module_name: String,
    version: String,
    scope: CoverScope,
    bin_paths: Vec<PathBuf>,
    env_vars: HashMap<String, String>,
    applied_at: chrono::DateTime<Utc>,
}

fn stack_path() -> PathBuf {
    fs::envswitch_home().join("state").join("stack.json")
}

fn load_stack() -> Vec<StackEntry> {
    let path = stack_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_stack(stack: &[StackEntry]) {
    let path = stack_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(stack).unwrap_or_default();
    let _ = std::fs::write(&path, data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_home() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("envswitch_test_{}", n));
        let _ = fs::remove_dir_all(&dir);
        // Clear any leftover state from previous test in same thread
        let _ = fs::remove_dir_all(&std::env::temp_dir().join("envswitch_state"));
        let _ = fs::remove_dir_all(&dir);
        std::env::set_var("ENVSWITCH_HOME", &dir);
        // Create mock install dirs
        for (mod_name, ver) in &[("jdk", "21"), ("jdk", "17"), ("go", "1.22")] {
            let p = dir.join("envs").join(mod_name).join(ver).join("bin");
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join("java"), b"fake").unwrap();
            fs::write(p.join("go"), b"fake").unwrap();
        }
        dir
    }

    fn teardown(dir: &std::path::Path) {
        let _ = fs::remove_dir_all(dir);
        std::env::remove_var("ENVSWITCH_HOME");
    }

    #[test]
    fn test_cover_adds_to_stack() {
        let dir = setup_test_home();
        let result = cover("jdk", "21", CoverScope::Session);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("JAVA_HOME"));
        assert!(script.contains("ENVSWITCH_SAVED_PATH"));
        let status = get_status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].module_name, "jdk");
        assert_eq!(status[0].version, "21");
        teardown(&dir);
    }

    #[test]
    fn test_cover_same_version_noop() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        // Second cover of same version should still succeed but log no-op
        let result = cover("jdk", "21", CoverScope::Session);
        assert!(result.is_ok());
        // Stack should still have only one entry
        assert_eq!(get_status().len(), 1);
        teardown(&dir);
    }

    #[test]
    fn test_cover_new_version_replaces() {
        let dir = setup_test_home();
        cover("jdk", "17", CoverScope::Session).unwrap();
        cover("jdk", "21", CoverScope::Session).unwrap();
        let status = get_status();
        // Only one jdk, should be 21
        let jdk: Vec<_> = status.iter().filter(|c| c.module_name == "jdk").collect();
        assert_eq!(jdk.len(), 1);
        assert_eq!(jdk[0].version, "21");
        teardown(&dir);
    }

    #[test]
    fn test_multi_module_stack() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        cover("go", "1.22", CoverScope::Session).unwrap();
        let status = get_status();
        assert_eq!(status.len(), 2);
        assert_eq!(status[0].module_name, "jdk"); // first covered
        assert_eq!(status[1].module_name, "go");   // second covered
        teardown(&dir);
    }

    #[test]
    fn test_uncover_removes_from_stack() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        cover("go", "1.22", CoverScope::Session).unwrap();
        let result = uncover("jdk");
        assert!(result.is_ok());
        let status = get_status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].module_name, "go");
        teardown(&dir);
    }

    #[test]
    fn test_uncover_all_clears_stack() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        cover("go", "1.22", CoverScope::Session).unwrap();
        let result = uncover_all();
        assert!(result.is_ok());
        assert!(get_status().is_empty());
        teardown(&dir);
    }

    #[test]
    fn test_full_rebuild_contains_all_vars() {
        let dir = setup_test_home();
        let _script = cover("jdk", "21", CoverScope::Session).unwrap();
        let script2 = cover("go", "1.22", CoverScope::Session).unwrap();
        assert!(script2.contains("JAVA_HOME"));
        assert!(script2.contains("GOROOT"));
        assert!(script2.contains("ENVSWITCH_SAVED_PATH"));
        // PATH should have both jdk and go
        assert!(script2.contains("jdk/21"));
        assert!(script2.contains("go/1.22"));
        teardown(&dir);
    }

    #[test]
    fn test_uncover_restores_path() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        let script = uncover("jdk").unwrap();
        // After uncover, PATH should reference only SAVED_PATH
        assert!(script.contains("ENVSWITCH_SAVED_PATH"));
        assert!(!script.contains("jdk/21"));
        assert!(get_status().is_empty());
        teardown(&dir);
    }
}
