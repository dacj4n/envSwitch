[中文版](README_zh.md)

# envSwitch

Fast development environment version manager for macOS and Linux.  
CLI + Desktop GUI (Tauri v2 + React + Tailwind). Zero Docker, zero compilation.

```bash
$ envswitch search jdk
  21.0.11  17.0.19  8.0.492

$ envswitch install jdk 21.0.11
$ envswitch cover jdk 21.0.11
$ java -version  # OpenJDK 21 ✓

$ envswitch cover go 1.26.3
$ go version     # Go 1.26.3 ✓
```

## Why envSwitch?

| | envSwitch | Docker | NVM/pyenv | ServBay |
|---|---|---|---|---|
| **Speed** | Native binary | VM overhead | Shell scripts | Native |
| **Install** | One click / command | `docker pull` | Script | GUI |
| **Switch** | Instant (symlinks) | Container restart | Shell eval | GUI click |
| **New terminal** | Auto (`.zshrc`) | Manual | Script in rc | Manual |
| **Services** | MySQL, PostgreSQL | Yes | No | Yes |
| **GUI** | Yes (Tauri v2) | — | — | Yes |
| **Footprint** | ~3MB | GBs | ~50MB | ~200MB |
| **i18n** | Chinese / English | — | — | — |

## Install

```bash
git clone https://github.com/your/envswitch.git
cd envswitch
cargo build --release -p envswitch --bin envswitch
sudo cp target/release/envswitch /usr/local/bin/

# One-time setup (auto-writes .zshrc)
envswitch init zsh
exec zsh
```

Or build the GUI:

```bash
cd gui
npm install
npx tauri build
open target/release/bundle/macos/envswitch.app
```

Requirements: `curl`, `rust`, [Homebrew](https://brew.sh) (for php/python/mysql/pgsql), `fnm`/`nvm` (optional, for node).

### Linux

Install [Linuxbrew](https://docs.brew.sh/Homebrew-on-Linux) — all modules work identically:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
```

## Supported Modules

| Module | Source | Type | Install |
|--------|--------|------|---------|
| **jdk** (8~25) | Azul Zulu API | SDK | Download + SHA256 |
| **go** (1.2~1.26) | go.dev API | SDK | Download + SHA256 |
| **node** (v0~v24) | nodejs.org API | SDK | Download + SHA256 |
| **php** (5.6~8.4) | Homebrew | SDK | `brew install` + shim |
| **python** (3.9~3.14) | Homebrew | SDK | `brew install` + shim |
| **mysql** (8.0~9.x) | Homebrew | Service | `brew install` + shim |
| **pgsql** (12~18) | Homebrew | Service | `brew install` + shim |

## Usage

```bash
# ── Search ──────────────────────────────────
envswitch search jdk           # Azul API
envswitch search go            # go.dev API
envswitch search node          # nodejs.org API
envswitch search php           # brew search
envswitch search python        # brew search
envswitch search mysql         # brew search
envswitch search pgsql         # brew search

# ── Install ─────────────────────────────────
envswitch install jdk 21.0.11
envswitch install go 1.26.3
envswitch install node 24.11.0
envswitch install php 8.3      # brew install php@8.3
envswitch install python 3.14  # brew install python@3.14
envswitch install mysql 8.0    # brew install mysql@8.0
envswitch install pgsql 16     # brew install postgresql@16

# ── Link (register existing installation) ───
envswitch link jdk 8 /Library/Java/JavaVirtualMachines/jdk1.8.0.jdk/Contents/Home

# ── List ────────────────────────────────────
envswitch list                 # all modules
envswitch list jdk             # jdk only

# ── Switch ──────────────────────────────────
envswitch cover jdk 21         # instant (shim symlink)
envswitch cover go 1.26.3
envswitch cover php 8.3
envswitch cover node 24
envswitch cover python 3.14

# ── Status ──────────────────────────────────
envswitch status               # current cover stack

# ── Uncover ─────────────────────────────────
envswitch uncover jdk          # restore system default
envswitch uncover --all        # restore all

# ── Services ────────────────────────────────
envswitch start mysql 8.0.46   # start MySQL
envswitch start pgsql 16.14    # start PostgreSQL
envswitch stop mysql           # stop service
envswitch stop pgsql
envswitch service-status       # all services
envswitch logs mysql --lines 100

# ── Global ──────────────────────────────────
envswitch cover jdk 21 --global  # persist across terminals

# ── Doctor ──────────────────────────────────
envswitch doctor               # diagnose setup issues

# ── cd-hook ─────────────────────────────────
envswitch cd-hook on           # auto-switch on cd into .envswitchrc dirs

# ── Project ─────────────────────────────────
cd my-project
envswitch init-project         # create .envswitchrc template
envswitch auto                 # apply project config
```

## Commands

| Command | Description |
|---------|-------------|
| `search <mod> [-r]` | Search available versions (local or remote) |
| `install <mod> <ver> [--force]` | Install a version |
| `uninstall <mod> <ver> [--purge]` | Uninstall a version (--purge removes data) |
| `link <mod> <ver> <path>` | Register an existing installation |
| `list [mod]` | List installed versions |
| `cover <mod> <ver> [--global]` | Activate a version (instant shim switch) |
| `uncover <mod>` | Deactivate a version |
| `uncover --all` | Deactivate all |
| `status` | Show current cover stack |
| `start <mod> <ver>` | Start a service |
| `stop <mod>` | Stop a service |
| `service-status` | Show service states |
| `logs <mod> [--lines N]` | View service logs |
| `init [zsh\|bash]` | Setup shell integration |
| `doctor` | Diagnose setup issues |
| `cd-hook <on\|off>` | Toggle auto-switch on `cd` |
| `init-project` | Create `.envswitchrc` |
| `auto` | Apply `.envswitchrc` |

## How It Works

envSwitch uses a **shims directory** (`~/.envswitch/shims/`) added to PATH once during `init`. When you `cover` a version, envSwitch creates symlinks in shims pointing to the active version's binaries. No `eval`, no shell hacks — filesystem symlinks work instantly in all terminals.

The GUI uses the same mechanism — clicking "Cover" updates the symlinks, and all terminals pick up the change.

```
~/.envswitch/
├── shims/          # Symlinks to active versions (→ in PATH)
├── envs/           # Installed versions
│   ├── jdk/21.0.11/
│   ├── php/8.3.31/
│   └── mysql/8.0.46/
├── data/           # Service data (per-version)
├── state/          # Cover stack persistence
├── cache/          # Download & metadata cache
├── logs/           # Operation logs
├── config/         # cd-hook, proxy config
├── config.json     # Proxy & settings
├── init.sh         # Shell integration (auto-generated)
└── tmp/            # Staging directory for atomic installs
```

## GUI

Built with **Tauri v2 + React + TypeScript + Tailwind CSS**:

```bash
cd gui
npm install
npx tauri dev     # development (hot reload)
npx tauri build   # production build → .app / .dmg
```

### Features

- **Module management** — list, search, install, uninstall, cover, uncover all modules
- **Service management** — start/stop MySQL, PostgreSQL with spinner feedback
- **Real-time install logs** — streaming curl progress bars, brew output in install window
- **Cancel with cleanup** — three-layer abort (token + process kill + tmp staging rollback)
- **Operation log** — timestamped history of all actions (install, cover, start, stop)
- **Auto-sync** — detects Homebrew/system JDK, Go, Node, Python and links automatically
- **Proxy support** — HTTP/HTTPS proxy applied to all curl and brew commands
- **Chinese/English** — full i18n with 100+ translation keys

### Pages

- **Versions** — module list with expandable cards, cover/uncover/install/uninstall per version
- **Services** — MySQL/PostgreSQL cards with status, metadata, start/stop
- **Status** — environment cover stack overview
- **Logs** — global operation log with level filters (OK/INFO/WARN/ERR)
- **Doctor** — diagnostic checks (platform, brew, modules, shims)
- **Settings** — language toggle, proxy configuration, CLI examples

## Proxy

Set a proxy for downloads (curl, brew):

```bash
# CLI: set in config file
echo '{"proxy": "http://127.0.0.1:7890"}' > ~/.envswitch/config.json

# GUI: Settings → Proxy → Save
```

The proxy is applied to all network operations: JDK/Go/Node downloads, Homebrew search/install, version APIs.

## Development

```bash
# CLI
cargo build -p envswitch --bin envswitch --release
cargo test -- --test-threads=1

# GUI
cd gui && npx tauri dev

# Workspace structure
.
├── Cargo.toml          # Workspace root (CLI + lib)
├── src/                # envswitch library + CLI binary
│   ├── infra/          # fs, download, oplog
│   ├── providers/      # jdk, go, node, php, python, mysql, pgsql, homebrew
│   └── config.rs       # Proxy & settings
└── gui/
    ├── src/            # React frontend (pages, components, i18n)
    └── src-tauri/      # Tauri backend (commands, job system)
```

## License

MIT
