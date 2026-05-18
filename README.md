[中文版](README_zh.md)

# envSwitch

**The native dev-environment version manager**. Switch Java, Go, Node, PHP, Python, MySQL, PostgreSQL versions in one keystroke — no containers, no shell hacks, no slow eval scripts.

[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)](https://github.com/dacj4n/envswitch)
[![Rust](https://img.shields.io/badge/rust-1.95+-orange)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

## Description

envSwitch installs and switches SDK/service versions using **filesystem symlinks via a shims directory** — add it to `$PATH` once, and every `cover` / `uncover` takes effect instantly across all terminals. No `eval`, no sub-shells, no PATH manipulation gymnastics.

- **CLI**: fast Rust binary (~3 MB), familiar `envswitch <cmd>` interface
- **GUI**: Tauri v2 + React + Tailwind CSS, one-click cover/install/service toggle
- **Modules**: JDK (Azul Zulu), Go (go.dev), Node (nodejs.org), PHP/Python/MySQL/PostgreSQL (Homebrew)
- **Services**: start/stop MySQL and PostgreSQL with per-version data isolation
- **i18n**: Chinese / English, 100+ translation keys

```bash
$ envswitch search jdk
  25.0.2  21.0.11  17.0.19  8.0.492

$ envswitch install jdk 21.0.11     # download + verify + extract — one command
$ envswitch cover jdk 21.0.11       # instant symlink switch
$ java -version                     # OpenJDK 21 ✓
```

## Install

### macOS — Homebrew

```bash
brew tap dacj4n/envswitch
brew install envswitch

# One-time shell setup
envswitch init zsh && source ~/.zshrc
```

### macOS — From source

```bash
git clone https://github.com/dacj4n/envswitch.git
cd envswitch
cargo build --release -p envswitch
sudo cp target/release/envswitch /usr/local/bin/
envswitch init zsh && source ~/.zshrc
```

### macOS — GUI

```bash
cd gui
npm install
npx tauri build
open src-tauri/target/release/bundle/macos/envswitch.app
```

### Linux — From source

```bash
git clone https://github.com/dacj4n/envswitch.git
cd envswitch
cargo build --release -p envswitch
sudo cp target/release/envswitch /usr/local/bin/

# Install Linuxbrew for php/python/mysql/pgsql modules:
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"

envswitch init bash && source ~/.bashrc
```

**Requirements**: `curl`, [Rust](https://rustup.rs) (from-source only), [Homebrew](https://brew.sh) (for php/python/mysql/pgsql), `fnm`/`nvm` (optional, node auto-detection).

## Supported Modules

| Module | Source | Versions | Type | Install |
|--------|--------|----------|------|---------|
| **jdk** | Azul Zulu API | 8 ~ 25 | SDK | Download + SHA256 |
| **go** | go.dev API | 1.2 ~ 1.26 | SDK | Download + SHA256 |
| **node** | nodejs.org API | v0 ~ v24 | SDK | Download + SHA256 |
| **php** | Homebrew | 5.6 ~ 8.4 | SDK | `brew install` + shim |
| **python** | Homebrew | 3.9 ~ 3.14 | SDK | `brew install` + shim |
| **mysql** | Homebrew | 8.0 ~ 9.x | Service | `brew install` + shim |
| **pgsql** | Homebrew | 12 ~ 18 | Service | `brew install` + shim |

## Quick Start

```bash
# ── Search ──────────────────────────────────
envswitch search jdk               # Azul Zulu metadata API
envswitch search go                # go.dev download page
envswitch search node              # nodejs.org dist API
envswitch search php               # brew search
envswitch search python            # brew search
envswitch search mysql             # brew search
envswitch search pgsql             # brew search

# ── Install ─────────────────────────────────
envswitch install jdk 21.0.11
envswitch install go 1.26.3
envswitch install node 24.11.0
envswitch install php 8.3          # → brew install php@8.3
envswitch install python 3.14      # → brew install python@3.14
envswitch install mysql 8.0        # → brew install mysql@8.0
envswitch install pgsql 16         # → brew install postgresql@16

# ── Link (register existing install) ────────
envswitch link jdk 8 /Library/Java/JavaVirtualMachines/jdk1.8.0.jdk/Contents/Home

# ── List ────────────────────────────────────
envswitch list                     # all modules
envswitch list jdk                 # jdk only

# ── Switch (instant — symlinks) ─────────────
envswitch cover jdk 21.0.11
envswitch cover go 1.26.3
envswitch cover php 8.3
envswitch cover node 24.11.0
envswitch cover python 3.14

# ── Status ──────────────────────────────────
envswitch status                   # current cover stack

# ── Uncover ─────────────────────────────────
envswitch uncover jdk              # restore system default
envswitch uncover --all            # restore all

# ── Services ────────────────────────────────
envswitch start mysql 8.0.46
envswitch start pgsql 16.14
envswitch stop mysql
envswitch stop pgsql
envswitch service-status           # all services at a glance
envswitch logs mysql --lines 100

# ── Global (persists across terminals) ──────
envswitch cover jdk 21.0.11 --global

# ── Doctor ──────────────────────────────────
envswitch doctor                   # full diagnostic report

# ── cd-hook (auto-switch on .envswitchrc) ──
envswitch cd-hook on

# ── Project ─────────────────────────────────
cd my-project
envswitch init-project             # create .envswitchrc template
envswitch auto                     # apply project env
```

## Commands

| Command | Description |
|---------|-------------|
| `search <mod> [-r]` | Search available versions (API / brew) |
| `install <mod> <ver> [--force]` | Download & install a version |
| `uninstall <mod> <ver> [--purge]` | Remove version (--purge also removes data) |
| `link <mod> <ver> <path>` | Register an existing installation |
| `list [mod]` | List installed versions |
| `cover <mod> <ver> [--global]` | Activate version (instant) |
| `uncover <mod>` | Deactivate version |
| `uncover --all` | Deactivate all |
| `status` | Show cover stack |
| `start <mod> <ver>` | Start a service |
| `stop <mod>` | Stop a service |
| `service-status` | Show service states |
| `logs <mod> [--lines N]` | View service logs |
| `init [zsh\|bash]` | Add shell integration to rc file (auto-detect if omitted) |
| `uninit [zsh\|bash]` | Remove shell integration from rc file |
| `init-status` | Check which shells have envswitch integration |
| `load-globals` | Output shell env for global covers (used by init.sh) |
| `doctor` | Diagnose setup issues |
| `cd-hook <on\|off>` | Toggle auto-switch on `cd` |
| `init-project` | Create `.envswitchrc` template |
| `auto` | Apply `.envswitchrc` |

## Architecture

envSwitch uses a **shims directory** (`~/.envswitch/shims/`) — added to `PATH` once during `init`. When you `cover` a version, envSwitch creates a symlink in `shims/` pointing to the target version's binary. All terminals see the change instantly because the filesystem symlink is updated, not the shell environment.

```
~/.envswitch/
├── shims/              # Symlinks to active version binaries (→ $PATH)
│   ├── java  → …/envs/jdk/21.0.11/bin/java
│   ├── go    → …/envs/go/1.26.3/bin/go
│   └── …
├── envs/               # Installed versions
│   ├── jdk/
│   │   ├── 21.0.11/    # ← symlink to extracted JDK
│   │   └── 8.0.492/
│   ├── go/1.26.3/
│   ├── php/8.3.31/
│   ├── python/3.14.0/
│   ├── mysql/8.0.46/
│   └── pgsql/16.14/
├── data/               # Per-version service data (MySQL/PG data dirs)
├── state/              # Cover stack (stack.json)
├── cache/              # Download & API response cache
├── logs/               # Global operation log (operations.log)
├── config/             # cd-hook state
├── config.json         # Proxy & user settings
├── init.sh             # Shell integration (auto-generated by `init`)
└── tmp/                # Staging directory for atomic installs
```

## GUI

Built with **Tauri v2 + React 19 + TypeScript + Tailwind CSS v4**.

```bash
cd gui
npm install
npx tauri dev     # development with hot reload
npx tauri build   # production → .app / .dmg
```

### Features

| Feature | Detail |
|---------|--------|
| **Module management** | List, search, install, uninstall, cover, uncover — all from expandable cards |
| **Non-blocking install** | Background jobs with real-time progress (curl `#` bar, brew output) |
| **Cancel + rollback** | Three-layer abort: cancellation token, process kill, tmp-staging cleanup |
| **Service management** | Start/stop MySQL, PostgreSQL with per-service spinner feedback |
| **Operation log** | Timestamped history of every action (install, cover, start, stop) |
| **Auto-sync** | Detects Homebrew/System JDK, Go, Node, Python and links automatically |
| **Proxy support** | HTTP/HTTPS proxy applied to all curl & brew network requests |
| **i18n** | Full Chinese (zh) / English (en) — 100+ translation keys |

### Pages

- **Versions** — Expandable module cards, installed + available versions, one-click cover/install
- **Services** — MySQL/PostgreSQL cards with status badge, PID, port, data dir
- **Status** — Environment cover stack table with shim-path mapping
- **Logs** — Global operation log with level filters (OK / INFO / WARN / ERR)
- **Doctor** — Diagnostic checks (platform, brew, shims, modules)
- **Settings** — Language toggle, proxy config, CLI command reference

## Proxy

Download sources (Azul CDN, go.dev, nodejs.org, Homebrew) can be slow in some regions. Set a proxy:

```bash
# CLI
echo '{"proxy": "http://127.0.0.1:7897"}' > ~/.envswitch/config.json

# GUI
Settings → Proxy → enter URL → Save
```

The proxy is applied as `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` to every `curl` and `brew` process spawned by envSwitch.

## Why not Docker / NVM / pyenv / ServBay?

| | envSwitch | Docker | NVM/pyenv | ServBay |
|---|---|---|---|---|
| **Switch speed** | Instant (symlink) | Container restart | Shell eval (slow) | GUI click |
| **New terminal** | Auto | Manual | rc script | Manual |
| **GUI** | ✓ (Tauri, native) | — | — | ✓ (Electron, heavy) |
| **Services** | MySQL, PG | Yes | No | Yes |
| **Binary size** | ~3 MB | GBs | ~50 MB | ~200 MB |
| **Install source** | API + Homebrew | Docker Hub | Script | Bundled |
| **i18n** | zh / en | — | — | — |
| **Proxy** | Built-in | Config | Manual | Manual |

## Development

```bash
# CLI
cargo build -p envswitch --bin envswitch --release
cargo test -- --test-threads=1

# GUI
cd gui && npx tauri dev
```

**Workspace layout:**

```
.
├── Cargo.toml              # Workspace root
├── src/                    # envswitch library + CLI binary
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library crate (shared with GUI)
│   ├── cli.rs              # Clap command definitions
│   ├── environment.rs      # Cover/uncover stack management
│   ├── install.rs          # Install/uninstall dispatch
│   ├── shell.rs            # init.sh generation
│   ├── service_mgr.rs      # Service lifecycle
│   ├── config.rs           # Proxy & settings
│   ├── infra/
│   │   ├── download.rs     # curl-based download + SHA256
│   │   ├── fs.rs           # envswitch_home, metadata I/O
│   │   └── oplog.rs        # Global operation log writer/reader
│   └── providers/
│       ├── jdk.rs          # Azul Zulu metadata API
│       ├── go.rs           # go.dev download API
│       ├── node.rs         # nodejs.org dist API
│       ├── php.rs          # Homebrew PHP
│       ├── python.rs       # Homebrew Python
│       ├── mysql.rs        # Homebrew MySQL
│       ├── postgresql.rs   # Homebrew PostgreSQL
│       └── homebrew.rs     # Shared brew helpers
└── gui/
    ├── src/                # React frontend
    │   ├── pages/          # Versions, Services, Status, Logs, Doctor, Settings
    │   ├── components/     # Sidebar, TopBar
    │   └── i18n/           # en/zh translation resources
    └── src-tauri/          # Tauri Rust backend
        └── src/lib.rs      # All Tauri commands + Job system
```

## License

MIT
