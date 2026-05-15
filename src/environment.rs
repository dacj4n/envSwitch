use crate::domain::*;
use crate::infra::fs;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn cover(module_name: &str, version: &str, scope: CoverScope) -> Result<String, String> {
    let module = crate::module_repo::find_module(module_name)
        .ok_or_else(|| format!("Unknown module: {}", module_name))?;

    let install_path = fs::envswitch_home().join("envs").join(module_name).join(version);
    if !install_path.exists() {
        return Err(format!("{} {} is not installed.", module_name, version));
    }

    let mut stack = load_stack();
    let is_top = stack.last().map_or(false, |c| c.module_name == module_name && c.version == version);
    if is_top {
        eprintln!("{} {} is already the current version.", module_name, version);
        return Ok(render_env());
    }

    stack.retain(|c| c.module_name != module_name);

    let data_path = fs::envswitch_home().join("data").join(module_name);
    let mut bin_paths: Vec<PathBuf> = module.path_entries.iter()
        .map(|pe| install_path.join(pe))
        .filter(|p| p.exists())
        .collect();

    if module_name == "jdk" {
        let home = crate::install::find_jdk_home(&install_path);
        let bin = home.join("bin");
        if bin.exists() { bin_paths = vec![bin]; }
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
    update_shims(&stack);

    Ok(render_full_env(&stack))
}

pub fn uncover(module_name: &str) -> Result<String, String> {
    let mut stack = load_stack();
    if !stack.iter().any(|c| c.module_name == module_name) {
        eprintln!("{} is not currently covered.", module_name);
        return Ok(String::new());
    }
    stack.retain(|c| c.module_name != module_name);
    save_stack(&stack);
    update_shims(&stack);
    Ok(render_full_env(&stack))
}

pub fn render_env() -> String {
    render_full_env(&load_stack())
}

pub fn uncover_all() -> Result<String, String> {
    let stack: Vec<StackEntry> = vec![];
    save_stack(&stack);
    update_shims(&stack);
    Ok(render_full_env(&stack))
}

fn render_full_env(stack: &[StackEntry]) -> String {
    let mut script = String::new();
    let all_modules = crate::module_repo::builtin_modules();
    let mut all_var_names: Vec<String> = Vec::new();
    for m in &all_modules {
        for (key, _) in &m.env_vars {
            if !all_var_names.contains(key) { all_var_names.push(key.clone()); }
        }
    }
    if !all_var_names.is_empty() {
        script.push_str(&format!("unset {}\n", all_var_names.join(" ")));
    }
    for entry in stack {
        for (key, value) in &entry.env_vars {
            script.push_str(&format!("export {}={}\n", key, value));
        }
    }
    script
}

fn update_shims(stack: &[StackEntry]) {
    let shims_dir = fs::envswitch_home().join("shims");
    let _ = std::fs::create_dir_all(&shims_dir);
    // Clear all existing shims
    if let Ok(entries) = std::fs::read_dir(&shims_dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    // Create shims from stack (last covered = highest priority)
    let mut seen = HashSet::new();
    for entry in stack.iter().rev() {
        for bin_path in &entry.bin_paths {
            if bin_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(bin_path) {
                    for file in entries.flatten() {
                        let name = file.file_name().to_string_lossy().to_string();
                        if seen.insert(name.clone()) {
                            let _ = std::fs::remove_file(shims_dir.join(&name));
                            let _ = std::os::unix::fs::symlink(&file.path(), &shims_dir.join(&name));
                        }
                    }
                }
            }
        }
    }
}

pub fn get_status() -> Vec<ActiveCover> {
    load_stack().into_iter().map(|e| ActiveCover {
        module_name: e.module_name, version: e.version, scope: e.scope,
        injected_paths: e.bin_paths,
        injected_envs: e.env_vars.keys().cloned().collect(),
        applied_at: e.applied_at,
    }).collect()
}

// ── Stack persistence ─────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StackEntry {
    module_name: String, version: String, scope: CoverScope,
    bin_paths: Vec<PathBuf>, env_vars: HashMap<String, String>,
    applied_at: chrono::DateTime<Utc>,
}

fn stack_path() -> PathBuf { fs::envswitch_home().join("state").join("stack.json") }

fn load_stack() -> Vec<StackEntry> {
    let path = stack_path();
    if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_default()).unwrap_or_default()
    } else { Vec::new() }
}

fn save_stack(stack: &[StackEntry]) {
    let path = stack_path();
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, serde_json::to_string_pretty(stack).unwrap_or_default());
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
        std::env::set_var("ENVSWITCH_HOME", &dir);
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
        assert!(result.unwrap().contains("JAVA_HOME"));
        assert_eq!(get_status().len(), 1);
        teardown(&dir);
    }

    #[test]
    fn test_cover_same_version_noop() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        cover("jdk", "21", CoverScope::Session).unwrap();
        assert_eq!(get_status().len(), 1);
        teardown(&dir);
    }

    #[test]
    fn test_cover_new_version_replaces() {
        let dir = setup_test_home();
        cover("jdk", "17", CoverScope::Session).unwrap();
        cover("jdk", "21", CoverScope::Session).unwrap();
        let status = get_status();
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
        assert_eq!(get_status().len(), 2);
        teardown(&dir);
    }

    #[test]
    fn test_uncover_removes_from_stack() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        cover("go", "1.22", CoverScope::Session).unwrap();
        uncover("jdk").unwrap();
        assert_eq!(get_status().len(), 1);
        teardown(&dir);
    }

    #[test]
    fn test_uncover_all_clears_stack() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        uncover_all().unwrap();
        assert!(get_status().is_empty());
        teardown(&dir);
    }

    #[test]
    fn test_full_rebuild_contains_all_vars() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        let script2 = cover("go", "1.22", CoverScope::Session).unwrap();
        assert!(script2.contains("JAVA_HOME"));
        assert!(script2.contains("GOROOT"));
        teardown(&dir);
    }

    #[test]
    fn test_env_vars_cleared_on_uncover_all() {
        let dir = setup_test_home();
        cover("jdk", "21", CoverScope::Session).unwrap();
        let script = uncover_all().unwrap();
        assert!(script.contains("unset"), "should unset vars: {}", script);
        assert!(!script.contains("export JAVA_HOME"), "should not export: {}", script);
        teardown(&dir);
    }
}
