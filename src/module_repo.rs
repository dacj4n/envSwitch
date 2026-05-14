use crate::domain::*;

/// Returns built-in module definitions (metadata only, no provider reference needed
/// since we dispatch statically in install/cmd modules).
pub fn builtin_modules() -> Vec<Module> {
    vec![
        Module {
            name: "jdk".into(),
            display_name: "OpenJDK (Temurin)".into(),
            category: ModuleCategory::Sdk,
            env_vars: vec![("JAVA_HOME".into(), "{install_path}".into())],
            path_entries: vec!["bin".into(), "Contents/Home/bin".into()],
            default_port: None,
        },
        Module {
            name: "go".into(),
            display_name: "Go".into(),
            category: ModuleCategory::Sdk,
            env_vars: vec![("GOROOT".into(), "{install_path}".into())],
            path_entries: vec!["bin".into()],
            default_port: None,
        },
        Module {
            name: "mysql".into(),
            display_name: "MySQL".into(),
            category: ModuleCategory::Service,
            env_vars: vec![("MYSQL_HOME".into(), "{install_path}".into())],
            path_entries: vec!["bin".into()],
            default_port: Some(3306),
        },
    ]
}

pub fn find_module(name: &str) -> Option<Module> {
    builtin_modules().into_iter().find(|m| m.name == name)
}

pub fn find_by_category(category: &ModuleCategory) -> Vec<Module> {
    builtin_modules()
        .into_iter()
        .filter(|m| &m.category == category)
        .collect()
}
