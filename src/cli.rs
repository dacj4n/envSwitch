use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "envswitch")]
#[command(version = "0.1.0")]
#[command(about = "Fast development environment version switcher")]
#[command(long_about = r#"
envSwitch — Fast development environment version switcher

Manage multiple versions of JDK, Go, MySQL and more.
Switch between them instantly using shell eval protocol.

Examples:
  envswitch list                    List all supported modules
  envswitch search jdk              Show available JDK versions
  envswitch install jdk 21          Install JDK 21
  envswitch cover jdk 21            Activate JDK 21
  envswitch uncover jdk             Deactivate JDK
  envswitch start mysql 8.0         Start MySQL 8.0
  envswitch stop mysql              Stop MySQL
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all supported modules or installed versions for a specific module
    List {
        /// Module name (e.g. jdk, go, mysql). Omit to list all.
        module: Option<String>,
    },

    /// Search available versions (local or remote)
    Search {
        /// Module name
        module: String,
        /// Force refresh, ignore cache
        #[arg(long, short)]
        refresh: bool,
    },

    /// Install a specific version of a module
    Install {
        /// Module name
        module: String,
        /// Version to install
        version: String,
        /// Force reinstall even if already installed
        #[arg(long)]
        force: bool,
    },

    /// Uninstall a specific version of a module
    Uninstall {
        /// Module name
        module: String,
        /// Version to uninstall
        version: String,
        /// Also remove service data
        #[arg(long)]
        purge: bool,
    },

    /// Generate environment cover script (use with eval)
    Cover {
        /// Module name
        module: String,
        /// Version to activate
        version: String,
        /// Cover scope: --global persists across terminals
        #[arg(long)]
        global: bool,
    },

    /// Remove a module from environment cover stack (use with eval)
    Uncover {
        /// Module name to uncover (e.g. jdk, go, mysql)
        module: Option<String>,
        /// Uncover ALL active modules at once
        #[arg(long, short)]
        all: bool,
    },

    /// Show current cover status
    Status,

    /// Output env vars as shell script (for shell function use)
    Export {
        /// Module name
        module: String,
        /// Version to export env for
        version: String,
        /// Whether it's a global cover
        #[arg(long)]
        global: bool,
    },

    /// Start a service (MySQL, etc.)
    Start {
        /// Module name
        module: String,
        /// Version to start
        version: String,
    },

    /// Stop a service
    Stop {
        /// Module name
        module: String,
    },

    /// Show service status
    ServiceStatus,

    /// Show service logs
    Logs {
        /// Module name
        module: String,
        /// Number of lines (default: 50)
        #[arg(long, default_value = "50")]
        lines: usize,
    },

    /// Read .envswitchrc and generate environment script
    Auto,

    /// Create a .envswitchrc template in current directory
    InitProject,

    /// Generate shell integration script
    Init {
        /// Shell type: zsh or bash
        #[arg(default_value = "zsh")]
        shell: String,
    },

    /// Enable/disable auto cd-hook for .envswitchrc
    CdHook {
        /// on | off
        state: String,
    },

    /// Register an existing installation path as an envswitch version
    Link {
        /// Module name
        module: String,
        /// Version label (e.g. "8.2", "5.6.40")
        version: String,
        /// Path to the installed software root (containing bin/)
        path: String,
    },

    /// Check envswitch setup and diagnose issues
    Doctor,
}
