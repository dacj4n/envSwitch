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
            display_name: "MySQL (via Homebrew)".into(),
            category: ModuleCategory::Service,
            env_vars: vec![("MYSQL_HOME".into(), "{install_path}".into())],
            path_entries: vec!["bin".into()],
            default_port: Some(3306),
        },
        Module {
            name: "php".into(),
            display_name: "PHP (via Homebrew)".into(),
            category: ModuleCategory::Sdk,
            env_vars: vec![("PHP_HOME".into(), "{install_path}".into())],
            path_entries: vec!["bin".into(), "sbin".into()],
            default_port: None,
        },
        Module {
            name: "python".into(),
            display_name: "Python (via Homebrew)".into(),
            category: ModuleCategory::Sdk,
            env_vars: vec![("PYTHON_HOME".into(), "{install_path}".into())],
            path_entries: vec!["bin".into()],
            default_port: None,
        },
        Module {
            name: "pgsql".into(),
            display_name: "PostgreSQL (via Homebrew)".into(),
            category: ModuleCategory::Service,
            env_vars: vec![("PGDATA".into(), "{data_path}".into())],
            path_entries: vec!["bin".into()],
            default_port: Some(5432),
        },
        Module {
            name: "node".into(),
            display_name: "Node.js (via fnm)".into(),
            category: ModuleCategory::Sdk,
            env_vars: vec![("NODE_HOME".into(), "{install_path}".into())],
            path_entries: vec!["bin".into()],
            default_port: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_modules_have_jdk_go_mysql() {
        let modules = builtin_modules();
        let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"jdk"));
        assert!(names.contains(&"go"));
        assert!(names.contains(&"mysql"));
    }

    #[test]
    fn test_find_module_exists() {
        let m = find_module("jdk").unwrap();
        assert_eq!(m.name, "jdk");
        assert_eq!(m.category, ModuleCategory::Sdk);
    }

    #[test]
    fn test_find_module_not_found() {
        assert!(find_module("foobar").is_none());
    }

    #[test]
    fn test_find_by_category() {
        let sdks = find_by_category(&ModuleCategory::Sdk);
        let services = find_by_category(&ModuleCategory::Service);
        assert!(sdks.iter().any(|m| m.name == "jdk"));
        assert!(services.iter().any(|m| m.name == "mysql"));
        // mysql should not be in SDK
        assert!(!sdks.iter().any(|m| m.name == "mysql"));
    }

    #[test]
    fn test_jdk_has_java_home() {
        let jdk = find_module("jdk").unwrap();
        assert!(jdk.env_vars.iter().any(|(k, _)| k == "JAVA_HOME"));
    }

    #[test]
    fn test_mysql_is_service() {
        let mysql = find_module("mysql").unwrap();
        assert_eq!(mysql.category, ModuleCategory::Service);
        assert_eq!(mysql.default_port, Some(3306));
    }
}
