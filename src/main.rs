mod cli;
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
    let _ = infra::fs::ensure_dirs();

    let result = match cli.command {
        Commands::List { module } => cmd_list(module),
        Commands::Remote { module } => cmd_remote(&module),
        Commands::Install { module, version, force } => install::install(&module, &version, force),
        Commands::Uninstall { module, version, purge } => install::uninstall(&module, &version, purge),
        Commands::Cover { module, version, global } => {
            let scope = if global { CoverScope::Global } else { CoverScope::Session };
            cmd_cover(&module, &version, scope)
        }
        Commands::Uncover { module, all } => {
            if all {
                cmd_uncover_all()
            } else if let Some(m) = module {
                if module_repo::find_module(&m).is_none() {
                    Err(format!(
                        "Unknown module: '{}'.\nUsage: envswitch uncover <module>   or   envswitch uncover --all",
                        m
                    ))
                } else {
                    cmd_uncover(&m)
                }
            } else {
                let names: Vec<String> = environment::get_status().iter()
                    .map(|c| c.module_name.clone()).collect();
                if names.is_empty() {
                    Err(format!(
                        "No active covers.\nUsage: envswitch uncover <module>   or   envswitch uncover --all"
                    ))
                } else {
                    Err(format!(
                        "Specify a module to uncover.\nActive: {}\nUsage: envswitch uncover <module>   or   envswitch uncover --all",
                        names.join(", ")
                    ))
                }
            }
        }
        Commands::Status => cmd_status(),
        Commands::Export { module: _, version: _, global } => {
            let _scope = if global { CoverScope::Global } else { CoverScope::Session };
            Err("export command is handled by shell function".into())
        },
        Commands::Start { module, version } => service_mgr::start(&module, &version).map(|_| ()),
        Commands::Stop { module } => service_mgr::stop(&module),
        Commands::ServiceStatus => cmd_service_status(),
        Commands::Logs { module, lines } => cmd_logs(&module, lines),
        Commands::Auto => cmd_auto(),
        Commands::InitProject => project::init_project(&std::env::current_dir().unwrap()),
        Commands::Init { shell } => cmd_init(&shell),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// ── Command handlers ────────────────────────────────────────────────

fn cmd_list(module: Option<String>) -> Result<(), String> {
    let modules = module_repo::builtin_modules();
    match module {
        Some(name) => {
            let m = module_repo::find_module(&name)
                .ok_or_else(|| format!("Unknown module: {}", name))?;
            let status = environment::get_status();
            let active = status.iter().find(|c| c.module_name == name);
            println!("{} ({})", m.display_name, m.name);
            println!("  Category: {:?}", m.category);
            let installed = install::list_installed(&name)?;
            if installed.is_empty() {
                println!("  Installed versions: (none)");
                println!("  Run 'envswitch remote {}' to see available versions.", name);
            } else {
                println!("  Installed versions:");
                for v in &installed {
                    let marker = if active.map_or(false, |a| a.version == v.version) { " [active]" } else { "" };
                    println!("    {} ({}){}", v.version, v.install_path.display(), marker);
                }
            }
        }
        None => {
            let installed_all = install::list_all_installed()?;
            let status = environment::get_status();
            if modules.is_empty() {
                println!("No modules available.");
                return Ok(());
            }
            println!("{:<12} {:<25} {:<10} {}", "MODULE", "NAME", "TYPE", "VERSIONS");
            println!("{}", "-".repeat(70));
            for m in &modules {
                let active_version = status.iter()
                    .find(|c| c.module_name == m.name)
                    .map(|c| c.version.as_str());
                let ver_str: String = installed_all.iter()
                    .filter(|v| v.module_name == m.name)
                    .map(|v| if active_version == Some(v.version.as_str()) { format!("{}*", v.version) } else { v.version.clone() })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("{:<12} {:<25} {:<10} {}",
                    m.name, m.display_name, format!("{:?}", m.category),
                    if ver_str.is_empty() { "-".into() } else { ver_str }
                );
            }
            if installed_all.is_empty() {
                println!("\nNo versions installed. Run 'envswitch remote <module>' to browse.");
            }
        }
    }
    Ok(())
}

fn cmd_remote(module_name: &str) -> Result<(), String> {
    match module_name {
        "go" => {
            eprintln!("Fetching Go versions from go.dev...");
            let versions = providers::go::GoProvider::fetch_remote_versions()?;
            for v in &versions { println!("  {}", v.version); }
        }
        "jdk" => {
            eprintln!("Fetching JDK versions from Azul Zulu...");
            let versions = providers::jdk::JdkProvider::fetch_remote_versions()?;
            for v in &versions { println!("  {}", v); }
        }
        "mysql" => {
            let versions = providers::mysql::MySqlProvider::fetch_remote_versions()?;
            for v in &versions { println!("  {}", v); }
        }
        _ => return Err(format!("Unknown module: {}. Use 'envswitch list'.", module_name)),
    }
    Ok(())
}

fn cmd_cover(module_name: &str, version: &str, scope: CoverScope) -> Result<(), String> {
    let scope_str = match &scope {
        CoverScope::Session => "session",
        CoverScope::Global => "global",
    };
    eprintln!("{} {} covered ({})", module_name, version, scope_str);
    // Output the full rebuilt environment as shell script
    print!("{}", environment::cover(module_name, version, scope)?);
    Ok(())
}

fn cmd_uncover(module_name: &str) -> Result<(), String> {
    eprintln!("{} uncovered.", module_name);
    print!("{}", environment::uncover(module_name)?);
    Ok(())
}

fn cmd_uncover_all() -> Result<(), String> {
    eprintln!("All modules uncovered.");
    print!("{}", environment::uncover_all()?);
    Ok(())
}

fn cmd_status() -> Result<(), String> {
    let covers = environment::get_status();
    if covers.is_empty() {
        println!("No active covers.");
    } else {
        println!("Stack (order: last = highest priority in PATH):");
        println!("{:<3} {:<12} {:<12} {:<12} {}", "#", "MODULE", "VERSION", "SCOPE", "APPLIED");
        println!("{}", "-".repeat(70));
        for (i, c) in covers.iter().enumerate() {
            println!("{:<3} {:<12} {:<12} {:<12} {}",
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
    let all = service_mgr::status_all()?;
    if all.is_empty() {
        println!("No services configured.");
        return Ok(());
    }
    println!("{:<12} {:<12} {:<10} {}", "SERVICE", "STATUS", "PID", "PORT");
    println!("{}", "-".repeat(50));
    for (name, status) in &all {
        match &status.running {
            Some(s) => println!("{:<12} {:<12} {:<10} {}", name, "Running", s.pid, s.port),
            None => println!("{:<12} {:<12} {:<10} {}", name, "Stopped", "-", "-"),
        }
    }
    Ok(())
}

fn cmd_service_status_quiet() -> Result<(), String> {
    let all = service_mgr::status_all()?;
    for (name, status) in &all {
        match &status.running {
            Some(s) => println!("  {}: Running (PID: {}, Port: {})", name, s.pid, s.port),
            None => println!("  {}: Stopped", name),
        }
    }
    Ok(())
}

fn cmd_logs(module_name: &str, lines: usize) -> Result<(), String> {
    for line in &service_mgr::logs(module_name, lines)? {
        println!("{}", line);
    }
    Ok(())
}

fn cmd_auto() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Cannot get current dir: {}", e))?;
    let config = project::load_config(&cwd)?
        .ok_or_else(|| ".envswitchrc not found. Run 'envswitch init-project' to create one.".to_string())?;

    // Apply all covers to stack (suppress per-cover output by not printing)
    for (mod_name, version) in &config.dependencies {
        let _ = environment::cover(mod_name, version, CoverScope::Session);
    }

    let mod_list: Vec<String> = environment::get_status().iter()
        .map(|c| format!("{} {}", c.module_name, c.version)).collect();
    eprintln!("Project environment ready: {}", mod_list.join(", "));

    // Output final full env rebuilt from stack
    print!("{}", environment::render_env());
    Ok(())
}

fn cmd_init(_shell_type: &str) -> Result<(), String> {
    let bin_path = std::env::current_exe()
        .map_err(|e| format!("Cannot determine binary path: {}", e))?;

    let init_script = shell::render_init(&bin_path.to_string_lossy());
    let init_path = infra::fs::envswitch_home().join("init.sh");
    std::fs::write(&init_path, &init_script)
        .map_err(|e| format!("Cannot write init.sh: {}", e))?;

    eprintln!("Shell integration written to {}", init_path.display());
    eprintln!();
    eprintln!("Add this ONE line to your ~/.zshrc:");
    eprintln!();
    println!("source {}", init_path.display());
    Ok(())
}
