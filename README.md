[中文版](README_zh.md)

# envSwitch

Fast, lightweight development environment version manager for macOS and Linux.

Switch between JDK, Go, PHP, Python, MySQL, PostgreSQL versions instantly — no Docker, no compilation, no junk in your shell config.

```bash
$ envswitch search jdk
  21.0.11+10.0.LTS
  17.0.19+10
  8.0.492

$ envswitch install jdk 21.0.11+10.0.LTS
# downloads from Azul Zulu API, SHA256 verified

$ envswitch cover jdk 21.0.11+10.0.LTS
$ java -version  # JDK 21 ✓

$ envswitch uncover jdk
$ java -version  # back to system default
```

## Why envSwitch?

| | envSwitch | Docker | NVM/pyenv | ServBay |
|---|---|---|---|---|
| **Speed** | Native binary | VM overhead | Shell scripts | Native |
| **Install** | `brew install` | `docker pull` | Script | GUI |
| **Switch** | Instant (symlinks) | Container restart | Shell eval | Instant |
| **New terminal** | Auto (`.zshrc`) | Manual | Script in rc | Manual GUI |
| **Services** | Yes (MySQL, PgSQL) | Yes | No | Yes |
| **Isolation** | Per-version data | Container | Global | Global |
| **Footprint** | ~3MB binary | GBs | ~50MB | ~200MB |

## Install

```bash
git clone https://github.com/your/envswitch.git
cd envswitch
cargo build --release
sudo cp target/release/envswitch /usr/local/bin/

# One-time setup
envswitch init zsh
exec zsh
```

Requirements: `curl`, `rust` (build only).

For PHP, Python, MySQL, PostgreSQL modules, [Homebrew](https://brew.sh) is required.

### Linux

Install [Linuxbrew](https://docs.brew.sh/Homebrew-on-Linux) — all modules work identically to macOS:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
```

If `brew` is not found when installing a module, envSwitch will show the Linuxbrew install guide.

## Supported Modules

| Module | Source | Type |
|--------|--------|------|
| **jdk** | Azul Zulu API | SDK |
| **go** | go.dev API | SDK |
| **php** | Homebrew | SDK |
| **python** | Homebrew | SDK |
| **mysql** | Homebrew | Service |
| **pgsql** | Homebrew | Service |

## Usage

```bash
# Search available versions
envswitch search <module>

# Install a version
envswitch install <module> <version>

# List installed versions
envswitch list
envswitch list jdk

# Switch versions (instant)
envswitch cover jdk 21
envswitch cover go 1.25.10
envswitch cover php 8.3
envswitch cover python 3.14

# Show current stack
envswitch status

# Uncover (restore system default)
envswitch uncover jdk
envswitch uncover --all

# Services (MySQL, PostgreSQL)
envswitch start mysql 8.0.46
envswitch stop mysql
envswitch logs mysql --lines 100

# Global (persist across terminals)
envswitch cover jdk 21 --global

# Project config
cd my-project
envswitch init-project
# edit .envswitchrc, then:
envswitch auto
```

## How It Works

envSwitch uses a **shims directory** (`~/.envswitch/shims/`) that is added to your PATH once during `init`. When you `cover` a version, envSwitch creates symlinks in the shims directory pointing to the active version's binaries. No `eval`, no shell hacks — just filesystem symlinks that work instantly in all terminals.

```
~/.envswitch/
├── shims/          # Symlinks to active versions (→ in PATH)
├── envs/           # Installed versions
│   ├── jdk/21.0.11/
│   ├── php/8.3.31/
│   └── mysql/8.0.46/
├── data/           # Service data (per-version)
│   ├── mysql/8.0.46/
│   └── pgsql/16.14/
├── state/          # Cover stack persistence
├── cache/          # Download cache
└── init.sh         # Shell integration (auto-generated)
```

## Commands

| Command | Description |
|---------|-------------|
| `search <mod>` | List available versions |
| `install <mod> <ver>` | Install a version |
| `uninstall <mod> <ver>` | Uninstall a version |
| `list [mod]` | List installed versions |
| `cover <mod> <ver>` | Activate a version |
| `uncover <mod>` | Deactivate a version |
| `uncover --all` | Deactivate all |
| `status` | Show current cover stack |
| `start <mod> <ver>` | Start a service |
| `stop <mod>` | Stop a service |
| `service-status` | Show service status |
| `logs <mod>` | View service logs |
| `init` | Setup shell integration |
| `init-project` | Create `.envswitchrc` template |
| `auto` | Apply `.envswitchrc` config |

## Development

```bash
cargo build --release
cargo test -- --test-threads=1
```

## License

MIT
