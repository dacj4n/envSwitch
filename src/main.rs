// Module declarations (binary target — duplicated from lib.rs for compilation)
mod cli;
mod config;
mod domain;
mod environment;
mod infra;
mod install;
mod module_repo;
mod platform;
mod project;
mod providers;
mod service_mgr;
mod shell;

use clap::Parser;
use cli::{Cli, Commands};
use domain::CoverScope;

fn main() {
    let cli = Cli::parse();
    let _ = crate::infra::fs::ensure_dirs();

    let result = match cli.command {
        Commands::List { module } => cmd_list(module),
        Commands::Search { module, refresh } => cmd_search(&module, refresh),
        Commands::Install {
            module,
            version,
            force,
        } => crate::install::install(&module, &version, force),
        Commands::Uninstall {
            module,
            version,
            purge,
        } => crate::install::uninstall(&module, &version, purge),
        Commands::Cover {
            module,
            version,
            global,
        } => {
            let scope = if global {
                CoverScope::Global
            } else {
                CoverScope::Session
            };
            cmd_cover(&module, &version, scope)
        }
        Commands::Uncover { module, all } => {
            if all {
                cmd_uncover_all()
            } else if let Some(m) = module {
                if crate::module_repo::find_module(&m).is_none() {
                    Err(format!(
                        "Unknown module: '{}'.\nUsage: envswitch uncover <module>   or   envswitch uncover --all",
                        m
                    ))
                } else {
                    cmd_uncover(&m)
                }
            } else {
                let names: Vec<String> = crate::environment::get_status()
                    .iter()
                    .map(|c| c.module_name.clone())
                    .collect();
                if names.is_empty() {
                    Err("No active covers.\nUsage: envswitch uncover <module>   or   envswitch uncover --all".to_string())
                } else {
                    Err(format!(
                        "Specify a module to uncover.\nActive: {}\nUsage: envswitch uncover <module>   or   envswitch uncover --all",
                        names.join(", ")
                    ))
                }
            }
        }
        Commands::Status => cmd_status(),
        Commands::Export {
            module: _,
            version: _,
            global,
        } => {
            let _scope = if global {
                CoverScope::Global
            } else {
                CoverScope::Session
            };
            Err("export command is handled by shell function".into())
        }
        Commands::Start { module, version } => {
            crate::service_mgr::start(&module, &version).map(|_| ())
        }
        Commands::Stop { module } => crate::service_mgr::stop(&module),
        Commands::ServiceStatus => cmd_service_status(),
        Commands::Logs { module, lines } => cmd_logs(&module, lines),
        Commands::Auto => cmd_auto(),
        Commands::InitProject => crate::project::init_project(&std::env::current_dir().unwrap()),
        Commands::Init { shell } => cmd_init(shell.as_deref()),
        Commands::Uninit { shell } => cmd_uninit(shell.as_deref()),
        Commands::InitStatus => cmd_init_status(),
        Commands::CdHook { state } => cmd_cd_hook(&state),
        Commands::Link {
            module,
            version,
            path,
        } => cmd_link(&module, &version, &path),
        Commands::Doctor => cmd_doctor(),
        Commands::LoadGlobals => {
            print!("{}", crate::environment::render_global_env());
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// ── Command handlers ────────────────────────────────────────────────

fn cmd_list(module: Option<String>) -> Result<(), String> {
    let modules = crate::module_repo::builtin_modules();
    match module {
        Some(name) => {
            let m = crate::module_repo::find_module(&name)
                .ok_or_else(|| format!("Unknown module: {}", name))?;
            let status = crate::environment::get_status();
            let active = status.iter().find(|c| c.module_name == name);
            println!("{} ({})", m.display_name, m.name);
            println!("  Category: {:?}", m.category);
            let installed = crate::install::list_installed(&name)?;
            if installed.is_empty() {
                println!("  Installed versions: (none)");
                println!(
                    "  Run 'envswitch remote {}' to see available versions.",
                    name
                );
            } else {
                println!("  Installed versions:");
                for v in &installed {
                    let marker = if active.is_some_and(|a| a.version == v.version) {
                        " [active]"
                    } else {
                        ""
                    };
                    println!("    {} ({}){}", v.version, v.install_path.display(), marker);
                }
            }
        }
        None => {
            let installed_all = crate::install::list_all_installed()?;
            let status = crate::environment::get_status();
            if modules.is_empty() {
                println!("No modules available.");
                return Ok(());
            }
            println!("{:<12} {:<25} {:<10} VERSIONS", "MODULE", "NAME", "TYPE");
            println!("{}", "-".repeat(70));
            for m in &modules {
                let active_version = status
                    .iter()
                    .find(|c| c.module_name == m.name)
                    .map(|c| c.version.as_str());
                let ver_str: String = installed_all
                    .iter()
                    .filter(|v| v.module_name == m.name)
                    .map(|v| {
                        if active_version == Some(v.version.as_str()) {
                            format!("{}*", v.version)
                        } else {
                            v.version.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "{:<12} {:<25} {:<10} {}",
                    m.name,
                    m.display_name,
                    format!("{:?}", m.category),
                    if ver_str.is_empty() {
                        "-".into()
                    } else {
                        ver_str
                    }
                );
            }
            if installed_all.is_empty() {
                println!("\nNo versions installed. Run 'envswitch remote <module>' to browse.");
            }
        }
    }
    Ok(())
}

fn cmd_search(module_name: &str, refresh: bool) -> Result<(), String> {
    if refresh {
        // Clear cache for the module
        let cache_dir = crate::infra::fs::envswitch_home().join("cache");
        let prefix = match module_name {
            "go" => "go_remote",
            "jdk" => "jdk_remote",
            _ => module_name,
        };
        // Remove all matching cache files
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    match module_name {
        "node" => {
            eprintln!("Fetching Node versions from nodejs.org...");
            let versions = crate::providers::node::NodeProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v.version);
            }
        }
        "go" => {
            eprintln!("Fetching Go versions from go.dev...");
            let versions = crate::providers::go::GoProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v.version);
            }
        }
        "jdk" => {
            eprintln!("Fetching JDK versions from Azul Zulu...");
            let versions = crate::providers::jdk::JdkProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v);
            }
        }
        "mysql" => {
            eprintln!("MySQL versions available via Homebrew:");
            let versions = crate::providers::mysql::MySqlProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v.version);
            }
        }
        "pgsql" => {
            eprintln!("PostgreSQL versions available via Homebrew:");
            let versions =
                crate::providers::postgresql::PostgresqlProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v.version);
            }
        }
        "php" => {
            eprintln!("PHP versions available via Homebrew:");
            let versions = crate::providers::php::PhpProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v.version);
            }
        }
        "python" => {
            eprintln!("Python versions available via Homebrew:");
            let versions = crate::providers::python::PythonProvider::fetch_remote_versions()?;
            for v in &versions {
                println!("  {}", v.version);
            }
        }
        _ => {
            return Err(format!(
                "Unknown module: {}. Use 'envswitch list'.",
                module_name
            ))
        }
    }
    Ok(())
}

fn cmd_cover(module_name: &str, version: &str, scope: CoverScope) -> Result<(), String> {
    crate::environment::cover(module_name, version, scope)
}

fn cmd_uncover(module_name: &str) -> Result<(), String> {
    crate::environment::uncover(module_name)
}

fn cmd_uncover_all() -> Result<(), String> {
    crate::environment::uncover_all()
}

fn cmd_status() -> Result<(), String> {
    let covers = crate::environment::get_status();
    if covers.is_empty() {
        println!("No active covers.");
    } else {
        println!("Stack (order: last = highest priority in PATH):");
        println!(
            "{:<3} {:<12} {:<12} {:<12} APPLIED",
            "#", "MODULE", "VERSION", "SCOPE"
        );
        println!("{}", "-".repeat(70));
        for (i, c) in covers.iter().enumerate() {
            println!(
                "{:<3} {:<12} {:<12} {:<12} {}",
                i + 1,
                c.module_name,
                c.version,
                format!("{:?}", c.scope),
                c.applied_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    println!();
    let _ = cmd_service_status_quiet();
    Ok(())
}

fn cmd_service_status() -> Result<(), String> {
    let all = crate::service_mgr::status_all()?;
    if all.is_empty() {
        println!("No services configured.");
        return Ok(());
    }
    println!("{:<12} {:<12} {:<10} PORT", "SERVICE", "STATUS", "PID");
    println!("{}", "-".repeat(50));
    for (name, status) in &all {
        match &status.running {
            Some(s) => println!("{:<12} {:<12} {:<10} {}", name, "Running", s.pid, s.port),
            None => println!("{:<12} {:<12} {:<10} -", name, "Stopped", "-"),
        }
    }
    Ok(())
}

fn cmd_service_status_quiet() -> Result<(), String> {
    let all = crate::service_mgr::status_all()?;
    for (name, status) in &all {
        match &status.running {
            Some(s) => println!("  {}: Running (PID: {}, Port: {})", name, s.pid, s.port),
            None => println!("  {}: Stopped", name),
        }
    }
    Ok(())
}

fn cmd_logs(module_name: &str, lines: usize) -> Result<(), String> {
    for line in &crate::service_mgr::logs(module_name, lines)? {
        println!("{}", line);
    }
    Ok(())
}

fn cmd_auto() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Cannot get current dir: {}", e))?;
    let config = crate::project::load_config(&cwd)?.ok_or_else(|| {
        ".envswitchrc not found. Run 'envswitch init-project' to create one.".to_string()
    })?;

    for (mod_name, version) in &config.dependencies {
        crate::environment::cover(mod_name, version, CoverScope::Session)?;
    }

    let mod_list: Vec<String> = crate::environment::get_status()
        .iter()
        .map(|c| format!("{} {}", c.module_name, c.version))
        .collect();
    eprintln!("Project environment ready: {}", mod_list.join(", "));
    Ok(())
}

fn home_dir() -> Result<std::path::PathBuf, String> {
    std::env::var("HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(dirs::home_dir)
        .ok_or("Cannot find home dir".to_string())
}

fn rc_path(shell: &str) -> Result<std::path::PathBuf, String> {
    let home = home_dir()?;
    Ok(match shell {
        "bash" => home.join(".bashrc"),
        _ => home.join(".zshrc"),
    })
}

fn detect_shell() -> &'static str {
    let home = home_dir().unwrap_or_default();
    if home.join(".bashrc").exists() {
        return "bash";
    }
    "zsh"
}

fn cmd_init(shell: Option<&str>) -> Result<(), String> {
    let shell = shell.unwrap_or_else(|| detect_shell());
    let bin_path =
        std::env::current_exe().map_err(|e| format!("Cannot determine binary path: {}", e))?;

    // Create shims and config directories
    let _ = std::fs::create_dir_all(crate::infra::fs::envswitch_home().join("shims"));
    let config_dir = crate::infra::fs::envswitch_home().join("config");
    let _ = std::fs::create_dir_all(&config_dir);
    let cd_hook_file = config_dir.join("cd-hook");
    if !cd_hook_file.exists() {
        let _ = std::fs::write(&cd_hook_file, "off");
    }

    // Write init.sh for backward compatibility (standalone file)
    let init_path = crate::infra::fs::envswitch_home().join("init.sh");
    let init_block = crate::shell::render_init_block(&bin_path.to_string_lossy());
    std::fs::write(&init_path, &init_block).map_err(|e| format!("Cannot write init.sh: {}", e))?;

    let rc_path = rc_path(shell)?;
    let content = std::fs::read_to_string(&rc_path).unwrap_or_default();

    // Idempotent: skip if already installed
    if crate::shell::has_init_block(&content) {
        eprintln!(
            "[INFO] Shell integration already exists in {}",
            rc_path.display()
        );
        return Ok(());
    }

    // Clean up old-style source lines and markers
    let clean = remove_old_init(&content, &init_path);

    // Append the init block at the end
    let new_content = format!("{}\n\n{}", clean.trim_end(), init_block);

    std::fs::write(&rc_path, &new_content)
        .map_err(|e| format!("Cannot write {}: {}", rc_path.display(), e))?;
    eprintln!("[OK] Added shell integration to {}", rc_path.display());
    eprintln!("[OK] Immediate effect: source {}", init_path.display());
    Ok(())
}

/// Remove old-style source lines from the rc content.
fn remove_old_init(content: &str, init_path: &std::path::Path) -> String {
    let source_line = format!("source {}", init_path.display());
    // Also strip any old block (just in case of partial/corrupt install)
    let content = crate::shell::remove_init_block(content);
    let clean: String = content
        .lines()
        .filter(|l| !l.contains(&source_line) && !l.trim().eq("# envSwitch"))
        .collect::<Vec<_>>()
        .join("\n");
    clean
}

fn cmd_uninit(shell: Option<&str>) -> Result<(), String> {
    let shell = shell.unwrap_or_else(|| detect_shell());
    let rc_path = rc_path(shell)?;
    let content = std::fs::read_to_string(&rc_path).unwrap_or_default();

    if !crate::shell::has_init_block(&content) {
        eprintln!("[INFO] No shell integration found in {}", rc_path.display());
        return Ok(());
    }

    let cleaned = crate::shell::remove_init_block(&content);
    std::fs::write(&rc_path, &cleaned)
        .map_err(|e| format!("Cannot write {}: {}", rc_path.display(), e))?;
    // Also clean up old source lines
    let init_path = crate::infra::fs::envswitch_home().join("init.sh");
    let cleaned = remove_old_init(&cleaned, &init_path);
    std::fs::write(&rc_path, &cleaned)
        .map_err(|e| format!("Cannot write {}: {}", rc_path.display(), e))?;
    eprintln!("[OK] Removed shell integration from {}", rc_path.display());
    eprintln!("Note: env files under ~/.envswitch/ are kept. Delete manually if desired.");
    eprintln!("To clean current shell session, open a new terminal or run:");
    eprintln!("  unset -f envswitch 2>/dev/null; unset _ENVSWITCH_BIN _ENVSWITCH_HOME _ENVSWITCH_LOADED 2>/dev/null");
    Ok(())
}

fn cmd_init_status() -> Result<(), String> {
    println!("Shell integration:");
    for &shell in &["zsh", "bash"] {
        if let Ok(rc) = rc_path(shell) {
            let content = std::fs::read_to_string(&rc).unwrap_or_default();
            let status = if crate::shell::has_init_block(&content) {
                "installed"
            } else {
                "not installed"
            };
            println!("  [{status}] {shell}rc ({})", rc.display());
        }
    }
    Ok(())
}

fn cmd_doctor() -> Result<(), String> {
    let home = crate::infra::fs::envswitch_home();
    let mut ok = 0;
    let mut warn = 0;
    let mut err = 0;

    macro_rules! check {
        ($label:expr, $cond:expr, $hint:expr) => {
            if $cond {
                println!("[ok] {}", $label);
                ok += 1;
            } else {
                println!("[!!] {} {}", $label, $hint);
                err += 1;
            }
        };
    }

    println!("envswitch doctor\n");
    println!("Home: {}", home.display());
    println!();

    check!(
        "shims directory exists",
        home.join("shims").is_dir(),
        "(run: envswitch init zsh)"
    );
    check!(
        "init.sh exists",
        home.join("init.sh").exists(),
        "(run: envswitch init zsh)"
    );

    // PATH check
    let path = std::env::var("PATH").unwrap_or_default();
    check!(
        "shims in PATH",
        path.contains("envswitch/shims"),
        "(add source ~/.envswitch/init.sh to ~/.zshrc)"
    );

    // zshrc check
    let zshrc = dirs::home_dir().unwrap_or_default().join(".zshrc");
    if zshrc.exists() {
        let content = std::fs::read_to_string(&zshrc).unwrap_or_default();
        check!(
            ".zshrc has shell integration",
            crate::shell::has_init_block(&content),
            "(run: envswitch init zsh)"
        );
    } else {
        println!("[--] .zshrc not found (non-zsh shell?)");
        warn += 1;
    }

    // brew check
    let has_brew = crate::providers::homebrew::brew_cmd()
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if has_brew {
        println!("[ok] Homebrew available");
        ok += 1;
    } else {
        let hint = if cfg!(target_os = "linux") {
            " (install Linuxbrew: https://docs.brew.sh/Homebrew-on-Linux)"
        } else {
            " (brew modules wont work: php/python/mysql/pgsql)"
        };
        println!("[--] Homebrew not found{}", hint);
        warn += 1;
    }

    // Installed modules
    println!();
    let modules = crate::module_repo::builtin_modules();
    for m in &modules {
        if let Ok(versions) = crate::install::list_installed(&m.name) {
            if versions.is_empty() {
                println!("[--] {}: no versions installed", m.name);
                warn += 1;
            } else {
                let ver_str: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
                println!("[ok] {}: {}", m.name, ver_str.join(", "));
                ok += 1;
            }
        }
    }

    // State files
    let stack_path = home.join("state").join("stack.json");
    if stack_path.exists() {
        if serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&stack_path).unwrap_or_default(),
        )
        .is_ok()
        {
            println!("[ok] state/stack.json valid");
            ok += 1;
        } else {
            println!("[!!] state/stack.json corrupted");
            err += 1;
        }
    }

    // Broken symlinks
    let envs_dir = home.join("envs");
    if envs_dir.is_dir() {
        let mut broken = 0;
        for entry in std::fs::read_dir(&envs_dir).into_iter().flatten().flatten() {
            if entry.path().is_symlink() && !entry.path().exists() {
                broken += 1;
                println!("[!!] broken symlink: {}", entry.path().display());
                err += 1;
            }
        }
        if broken == 0 {
            println!("[ok] no broken symlinks in envs/");
            ok += 1;
        }
    }

    println!();
    println!("{} ok, {} warnings, {} errors", ok, warn, err);
    if err > 0 {
        Err(format!(
            "{} issue(s) found. See above for fix instructions.",
            err
        ))
    } else {
        Ok(())
    }
}

fn cmd_link(module_name: &str, version: &str, path: &str) -> Result<(), String> {
    let src = std::path::PathBuf::from(path);
    if !src.exists() {
        return Err(format!("Path not found: {}", path));
    }
    if !src.join("bin").exists() && !src.join("Contents").join("Home").join("bin").exists() {
        return Err(format!(
            "No bin/ directory found at {}. Expected a software root.",
            path
        ));
    }

    let dest = crate::infra::fs::envswitch_home()
        .join("envs")
        .join(module_name)
        .join(version);
    if dest.exists() {
        return Err(format!(
            "{} {} is already installed at {}",
            module_name,
            version,
            dest.display()
        ));
    }

    let _ = std::fs::create_dir_all(dest.parent().unwrap());
    std::os::unix::fs::symlink(&src, &dest).map_err(|e| format!("symlink: {}", e))?;

    // Write metadata
    let meta_path = crate::infra::fs::envswitch_home()
        .join("envs")
        .join(module_name)
        .join("metadata.json");
    let mut meta = if meta_path.exists() {
        serde_json::from_str::<crate::domain::InstalledMetadata>(
            &std::fs::read_to_string(&meta_path).unwrap_or_default(),
        )
        .unwrap_or(crate::domain::InstalledMetadata { versions: vec![] })
    } else {
        crate::domain::InstalledMetadata { versions: vec![] }
    };
    meta.versions.retain(|v| v.install_path != dest);
    meta.versions.push(crate::domain::InstalledVersion {
        module_name: module_name.to_string(),
        version: version.to_string(),
        install_path: dest.clone(),
        installed_at: chrono::Utc::now(),
        size_bytes: 0,
        source: "custom".into(),
    });
    let _ = std::fs::create_dir_all(meta_path.parent().unwrap());
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("write metadata: {}", e))?;

    eprintln!("{} {} linked from {}", module_name, version, src.display());
    eprintln!("Now run: envswitch cover {} {}", module_name, version);
    Ok(())
}

fn cmd_cd_hook(state: &str) -> Result<(), String> {
    let config_dir = crate::infra::fs::envswitch_home().join("config");
    let _ = std::fs::create_dir_all(&config_dir);
    let hook_file = config_dir.join("cd-hook");

    match state {
        "on" => {
            std::fs::write(&hook_file, "on").map_err(|e| format!("write: {}", e))?;
            eprintln!("cd-hook enabled. Run: source ~/.zshrc   (or open new terminal)");
            eprintln!("Then cd into a dir with .envswitchrc to auto-switch.");
        }
        "off" => {
            std::fs::write(&hook_file, "off").map_err(|e| format!("write: {}", e))?;
            eprintln!("cd-hook disabled.");
        }
        _ => return Err("Usage: envswitch cd-hook on|off".to_string()),
    }
    Ok(())
}
