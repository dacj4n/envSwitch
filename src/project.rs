use crate::domain::{CoverScope, ProjectConfig};

/// Load `.envswitchrc` from a directory.
pub fn load_config(dir: &std::path::Path) -> Result<Option<ProjectConfig>, String> {
    let path = dir.join(".envswitchrc");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read .envswitchrc: {}", e))?;
    let config: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Invalid .envswitchrc YAML: {}", e))?;
    let mut dependencies = Vec::new();
    if let Some(deps) = config["dependencies"].as_sequence() {
        for dep in deps {
            if let Some(arr) = dep.as_sequence() {
                if arr.len() >= 2 {
                    let name = arr[0].as_str().unwrap_or("").to_string();
                    let version = arr[1].as_str().unwrap_or("").to_string();
                    if !name.is_empty() && !version.is_empty() {
                        dependencies.push((name, version));
                    }
                }
            }
        }
    }
    Ok(Some(ProjectConfig { dependencies }))
}

/// Apply all dependencies to the cover stack, then return the full env script from the stack.
pub fn compute_auto(config: &ProjectConfig, scope: &CoverScope) -> Result<String, String> {
    if config.dependencies.is_empty() {
        return Err("No dependencies in .envswitchrc. Edit the file to add some.".into());
    }
    for (module_name, version) in &config.dependencies {
        let module = crate::module_repo::find_module(module_name);
        match module {
            Some(m) if m.category == crate::domain::ModuleCategory::Service => {
                if let Err(e) = crate::service_mgr::start(module_name, version) {
                    eprintln!("Warning: failed to start {} {}: {}", module_name, version, e);
                }
            }
            _ => {}
        }
        // Apply to stack
        if let Err(e) = crate::environment::cover(module_name, version, scope.clone()) {
            eprintln!("Warning: {} {} : {}", module_name, version, e);
        }
    }
    Ok(crate::environment::render_env())
}

/// Create a template .envswitchrc file.
pub fn init_project(dir: &std::path::Path) -> Result<(), String> {
    let path = dir.join(".envswitchrc");
    if path.exists() {
        return Err(".envswitchrc already exists in this directory.".into());
    }
    let template = r#"# envSwitch project configuration
# dependencies:
#   - [module, version]
#
dependencies:
  # - [jdk, "21"]
  # - [go, "1.22"]
"#;
    std::fs::write(&path, template).map_err(|e| format!("Cannot create: {}", e))?;
    eprintln!("Created .envswitchrc");
    Ok(())
}
