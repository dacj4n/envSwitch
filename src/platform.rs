/// Platform detection — unified OS/arch mapping for all providers.

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    MacAarch64,
    MacX64,
    LinuxX64,
    LinuxAarch64,
    WindowsX64,
    WindowsAarch64,
}

impl Platform {
    /// Detect the current platform at compile time.
    pub fn current() -> Self {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => Platform::MacAarch64,
            ("macos", "x86_64") => Platform::MacX64,
            ("linux", "x86_64") => Platform::LinuxX64,
            ("linux", "aarch64") => Platform::LinuxAarch64,
            ("windows", "x86_64") => Platform::WindowsX64,
            ("windows", "aarch64") => Platform::WindowsAarch64,
            (os, arch) => panic!("Unsupported platform: {}/{}", os, arch),
        }
    }

    /// Display name for user-facing messages.
    pub fn display(&self) -> &str {
        match self {
            Platform::MacAarch64 => "macOS (Apple Silicon)",
            Platform::MacX64 => "macOS (Intel)",
            Platform::LinuxX64 => "Linux (x64)",
            Platform::LinuxAarch64 => "Linux (ARM64)",
            Platform::WindowsX64 => "Windows (x64)",
            Platform::WindowsAarch64 => "Windows (ARM64)",
        }
    }

    // ── Adoptium JDK API naming ─────────────────────────────────────

    pub fn adoptium_os(&self) -> &str {
        match self {
            Platform::MacAarch64 | Platform::MacX64 => "mac",
            Platform::LinuxX64 | Platform::LinuxAarch64 => "linux",
            Platform::WindowsX64 | Platform::WindowsAarch64 => "windows",
        }
    }

    pub fn adoptium_arch(&self) -> &str {
        match self {
            Platform::MacX64 | Platform::LinuxX64 | Platform::WindowsX64 => "x64",
            Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64 => "aarch64",
        }
    }

    // ── Go download naming ───────────────────────────────────────────

    pub fn go_os(&self) -> &str {
        match self {
            Platform::MacAarch64 | Platform::MacX64 => "darwin",
            Platform::LinuxX64 | Platform::LinuxAarch64 => "linux",
            Platform::WindowsX64 | Platform::WindowsAarch64 => "windows",
        }
    }

    pub fn go_arch(&self) -> &str {
        match self {
            Platform::MacX64 | Platform::LinuxX64 | Platform::WindowsX64 => "amd64",
            Platform::MacAarch64 | Platform::LinuxAarch64 | Platform::WindowsAarch64 => "arm64",
        }
    }

    // ── MySQL download naming ────────────────────────────────────────

    pub fn mysql_os_tag(&self) -> &str {
        match self {
            Platform::MacAarch64 => "macos14-arm64",
            Platform::MacX64 => "macos14-x86_64",
            Platform::LinuxX64 => "linux-glibc2.28-x86_64",
            Platform::LinuxAarch64 => "linux-glibc2.28-aarch64",
            Platform::WindowsX64 => "winx64",
            Platform::WindowsAarch64 => "winarm64",
        }
    }

    /// Parse a user-provided version string and suggest the closest download.
    pub fn mysql_version_dir(version: &str) -> String {
        // MySQL version grouping: 8.0.x → mysql-8.0/
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            format!("mysql-{}.{}", parts[0], parts[1])
        } else {
            format!("mysql-{}", version)
        }
    }
}
