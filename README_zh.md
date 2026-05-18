[English](README.md)

# envSwitch

**原生开发环境版本管理器**。一键切换 Java、Go、Node、PHP、Python、MySQL、PostgreSQL 版本 — 无需容器，无需 Shell Hack，无性能损耗。

[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)](https://github.com/dacj4n/envswitch)
[![Rust](https://img.shields.io/badge/rust-1.95+-orange)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

## 简介

envSwitch 通过 **shims 目录 + 文件系统符号链接** 来安装和切换 SDK/服务版本。将 shims 目录一次性加入 `$PATH`，每次 `cover` / `uncover` 在所有终端即时生效。无需 `eval`，无需子 Shell，无需 PATH 拼接黑魔法。

- **CLI**: 纯 Rust 二进制（~3 MB），`envswitch <命令>` 风格
- **GUI**: Tauri v2 + React + Tailwind CSS，一键切换/安装/启停服务
- **模块**: JDK (Azul Zulu)、Go (go.dev)、Node (nodejs.org)、PHP/Python/MySQL/PostgreSQL (Homebrew)
- **服务**: 启停 MySQL、PostgreSQL，按版本隔离数据
- **国际化**: 中文 / 英文，100+ 翻译键

```bash
$ envswitch search jdk
  25.0.2  21.0.11  17.0.19  8.0.492

$ envswitch install jdk 21.0.11     # 下载 + 校验 + 解压 — 一步到位
$ envswitch cover jdk 21.0.11       # 即时 symlink 切换
$ java -version                     # OpenJDK 21 ✓
```

## 安装

### macOS — Homebrew

```bash
brew tap dacj4n/envswitch
brew install envswitch

# 一次性 Shell 配置
envswitch init zsh && source ~/.zshrc
```

### macOS — 源码编译

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

### Linux — 源码编译

```bash
git clone https://github.com/dacj4n/envswitch.git
cd envswitch
cargo build --release -p envswitch
sudo cp target/release/envswitch /usr/local/bin/

# php/python/mysql/pgsql 模块需要安装 Linuxbrew：
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"

envswitch init bash && source ~/.bashrc
```

**依赖**：`curl`、[Rust](https://rustup.rs)（仅源码编译需要）、[Homebrew](https://brew.sh)（php/python/mysql/pgsql 需要）、`fnm`/`nvm`（可选，用于 node 自动检测）。

## 支持的模块

| 模块 | 来源 | 版本范围 | 类型 | 安装方式 |
|------|------|----------|------|---------|
| **jdk** | Azul Zulu API | 8 ~ 25 | SDK | 下载 + SHA256 |
| **go** | go.dev API | 1.2 ~ 1.26 | SDK | 下载 + SHA256 |
| **node** | nodejs.org API | v0 ~ v24 | SDK | 下载 + SHA256 |
| **php** | Homebrew | 5.6 ~ 8.4 | SDK | `brew install` + shim |
| **python** | Homebrew | 3.9 ~ 3.14 | SDK | `brew install` + shim |
| **mysql** | Homebrew | 8.0 ~ 9.x | 服务 | `brew install` + shim |
| **pgsql** | Homebrew | 12 ~ 18 | 服务 | `brew install` + shim |

## 快速上手

```bash
# ── 搜索 ──────────────────────────────────
envswitch search jdk               # Azul Zulu API
envswitch search go                # go.dev API
envswitch search node              # nodejs.org API
envswitch search php               # brew search
envswitch search python            # brew search
envswitch search mysql             # brew search
envswitch search pgsql             # brew search

# ── 安装 ─────────────────────────────────
envswitch install jdk 21.0.11
envswitch install go 1.26.3
envswitch install node 24.11.0
envswitch install php 8.3          # → brew install php@8.3
envswitch install python 3.14      # → brew install python@3.14
envswitch install mysql 8.0        # → brew install mysql@8.0
envswitch install pgsql 16         # → brew install postgresql@16

# ── 链接（注册已有安装） ──────────────────
envswitch link jdk 8 /Library/Java/JavaVirtualMachines/jdk1.8.0.jdk/Contents/Home

# ── 列表 ─────────────────────────────────
envswitch list                     # 所有模块
envswitch list jdk                 # 只看 jdk

# ── 切换（即时生效 — symlinks） ────────────
envswitch cover jdk 21.0.11
envswitch cover go 1.26.3
envswitch cover php 8.3
envswitch cover node 24.11.0
envswitch cover python 3.14

# ── 状态 ─────────────────────────────────
envswitch status                   # 当前覆盖栈

# ── 取消覆盖 ─────────────────────────────
envswitch uncover jdk              # 恢复系统默认
envswitch uncover --all            # 恢复全部

# ── 服务 ─────────────────────────────────
envswitch start mysql 8.0.46
envswitch start pgsql 16.14
envswitch stop mysql
envswitch stop pgsql
envswitch service-status           # 所有服务一览
envswitch logs mysql --lines 100

# ── 全局（新终端自动生效） ────────────────
envswitch cover jdk 21.0.11 --global

# ── 诊断 ─────────────────────────────────
envswitch doctor                   # 完整诊断报告

# ── cd 自动切换 ───────────────────────────
envswitch cd-hook on               # 进入含 .envswitchrc 的目录自动切换

# ── 项目配置 ─────────────────────────────
cd my-project
envswitch init-project             # 创建 .envswitchrc 模板
envswitch auto                     # 应用项目环境
```

## 命令列表

| 命令 | 说明 |
|------|------|
| `search <模块> [-r]` | 搜索可用版本（API / brew） |
| `install <模块> <版本> [--force]` | 下载并安装 |
| `uninstall <模块> <版本> [--purge]` | 卸载（--purge 同时删除数据） |
| `link <模块> <版本> <路径>` | 注册已有安装 |
| `list [模块]` | 列出已安装版本 |
| `cover <模块> <版本> [--global]` | 激活版本（即时生效） |
| `uncover <模块>` | 取消激活 |
| `uncover --all` | 取消全部 |
| `status` | 当前覆盖栈 |
| `start <模块> <版本>` | 启动服务 |
| `stop <模块>` | 停止服务 |
| `service-status` | 服务状态 |
| `logs <模块> [--lines N]` | 服务日志 |
| `init [zsh\|bash]` | 添加 Shell 集成到 rc 文件（可省略参数自动检测） |
| `uninit [zsh\|bash]` | 从 rc 文件移除 Shell 集成 |
| `init-status` | 查看各 Shell 的集成状态 |
| `load-globals` | 输出全局覆盖的 Shell 环境变量（供 init.sh 调用） |
| `doctor` | 诊断配置问题 |
| `cd-hook <on\|off>` | cd 自动切换开关 |
| `init-project` | 创建 `.envswitchrc` 模板 |
| `auto` | 应用 `.envswitchrc` |

## 工作原理

envSwitch 使用 **shims 目录**（`~/.envswitch/shims/`）— 在 `init` 时一次性加入 `$PATH`。执行 `cover` 时，envSwitch 在 `shims/` 中创建指向目标版本二进制的符号链接。因为是文件系统级操作，所有终端即时感知变化，无需重新加载 Shell。

```
~/.envswitch/
├── shims/              # 指向当前激活版本的 symlinks（已在 $PATH 中）
│   ├── java  → …/envs/jdk/21.0.11/bin/java
│   ├── go    → …/envs/go/1.26.3/bin/go
│   └── …
├── envs/               # 已安装版本
│   ├── jdk/
│   │   ├── 21.0.11/    # ← symlink 到解压后的 JDK 目录
│   │   └── 8.0.492/
│   ├── go/1.26.3/
│   ├── php/8.3.31/
│   ├── python/3.14.0/
│   ├── mysql/8.0.46/
│   └── pgsql/16.14/
├── data/               # 服务数据（按版本隔离）
├── state/              # 覆盖栈持久化（stack.json）
├── cache/              # 下载与 API 响应缓存
├── logs/               # 全局操作日志（operations.log）
├── config/             # cd-hook 状态
├── config.json         # 代理与用户设置
├── init.sh             # Shell 集成脚本（`init` 命令自动生成）
└── tmp/                # 原子化安装暂存目录
```

## GUI

基于 **Tauri v2 + React 19 + TypeScript + Tailwind CSS v4**。

```bash
cd gui
npm install
npx tauri dev     # 开发模式（热加载）
npx tauri build   # 生产构建 → .app / .dmg
```

### 功能

| 功能 | 说明 |
|------|------|
| **模块管理** | 列表、搜索、安装、卸载、覆盖、取消覆盖 — 全部通过展开式卡片操作 |
| **非阻塞安装** | 后台任务 + 实时进度（curl `#` 进度条、brew 输出） |
| **取消并清理** | 三层中断：取消令牌、进程终止、暂存目录回滚 |
| **服务管理** | 启停 MySQL/PostgreSQL，每服务独立转圈反馈 |
| **操作日志** | 所有操作（安装/覆盖/启停）带时间戳记录 |
| **自动同步** | 自动检测 Homebrew/系统 已安装的 JDK、Go、Node、Python |
| **代理支持** | HTTP/HTTPS 代理应用于所有 curl 和 brew 网络请求 |
| **国际化** | 完整中文/英文切换，100+ 翻译键 |

### 页面

- **版本** — 展开式模块卡片，已安装与可用版本，一键 Cover/Install
- **服务** — MySQL/PostgreSQL 卡片，含状态徽标、PID、端口、数据目录
- **状态** — 环境覆盖栈表格，含 shim 路径映射
- **日志** — 全局操作日志，按级别筛选（OK / INFO / WARN / ERR）
- **诊断** — 诊断检查（平台、brew、shims、模块）
- **设置** — 语言切换、代理配置、CLI 命令参考

## 代理

部分区域的下载源（Azul CDN、go.dev、nodejs.org、Homebrew）可能较慢。设置代理加速：

```bash
# CLI
echo '{"proxy": "http://127.0.0.1:7897"}' > ~/.envswitch/config.json

# GUI
设置 → 代理 → 输入 URL → 保存
```

代理以 `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` 环境变量注入到 envSwitch 启动的每个 `curl` 和 `brew` 进程中。

## 对比

| | envSwitch | Docker | NVM/pyenv | ServBay |
|---|---|---|---|---|
| **切换速度** | 即时（symlink） | 重启容器 | Shell eval（慢） | GUI 点击 |
| **新终端** | 自动 | 手动 | rc 脚本 | 手动 |
| **GUI** | ✓（Tauri，原生） | — | — | ✓（Electron，重） |
| **服务** | MySQL、PG | ✓ | ✗ | ✓ |
| **体积** | ~3 MB | GB 级 | ~50 MB | ~200 MB |
| **安装源** | API + Homebrew | Docker Hub | 脚本 | 内置 |
| **国际化** | zh / en | — | — | — |
| **代理** | 内置 | 配置 | 手动 | 手动 |

## 开发

```bash
# CLI
cargo build -p envswitch --bin envswitch --release
cargo test -- --test-threads=1

# GUI
cd gui && npx tauri dev
```

**项目结构：**

```
.
├── Cargo.toml              # 工作区根
├── src/                    # envswitch 库 + CLI 二进制
│   ├── main.rs             # CLI 入口
│   ├── lib.rs              # 库（GUI 共享）
│   ├── cli.rs              # Clap 命令定义
│   ├── environment.rs      # Cover/uncover 覆盖栈管理
│   ├── install.rs          # 安装/卸载调度
│   ├── shell.rs            # init.sh 生成
│   ├── service_mgr.rs      # 服务生命周期
│   ├── config.rs           # 代理与设置
│   ├── infra/
│   │   ├── download.rs     # curl 下载 + SHA256 校验
│   │   ├── fs.rs           # envswitch_home、元数据读写
│   │   └── oplog.rs        # 全局操作日志
│   └── providers/
│       ├── jdk.rs          # Azul Zulu API
│       ├── go.rs           # go.dev API
│       ├── node.rs         # nodejs.org API
│       ├── php.rs          # Homebrew PHP
│       ├── python.rs       # Homebrew Python
│       ├── mysql.rs        # Homebrew MySQL
│       ├── postgresql.rs   # Homebrew PostgreSQL
│       └── homebrew.rs     # 共享 brew 工具
└── gui/
    ├── src/                # React 前端
    │   ├── pages/          # Versions, Services, Status, Logs, Doctor, Settings
    │   ├── components/     # Sidebar, TopBar
    │   └── i18n/           # en/zh 翻译资源
    └── src-tauri/          # Tauri Rust 后端
        └── src/lib.rs      # 全部 Tauri 命令 + 任务系统
```

## 许可证

MIT
