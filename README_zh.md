# envSwitch

快速开发环境版本管理器，支持 macOS / Linux。  
CLI + 桌面 GUI (Tauri v2 + React + Tailwind)。零 Docker，零编译。

```bash
$ envswitch search jdk
  21.0.11  17.0.19  8.0.492

$ envswitch install jdk 21.0.11
$ envswitch cover jdk 21.0.11
$ java -version  # OpenJDK 21 ✓

$ envswitch cover go 1.26.3
$ go version     # Go 1.26.3 ✓
```

## 为什么用 envSwitch？

| 特性 | envSwitch | Docker | NVM/pyenv | ServBay |
|------|-----------|--------|-----------|---------|
| **速度** | 原生二进制 | 虚拟机开销 | Shell 脚本 | 原生 |
| **安装** | 一键 / 一行命令 | `docker pull` | 脚本 | GUI |
| **切换** | 即时（symlinks） | 重启容器 | Shell eval | GUI 点击 |
| **新终端** | 自动生效 | 手动 | 配置 rc | 手动 |
| **服务** | MySQL, PostgreSQL | ✓ | ✗ | ✓ |
| **GUI** | ✓ (Tauri v2) | — | — | ✓ |
| **体积** | ~3MB | GB 级 | ~50MB | ~200MB |
| **国际化** | 中文 / 英文 | — | — | — |

## 安装

```bash
git clone https://github.com/your/envswitch.git
cd envswitch
cargo build --release -p envswitch --bin envswitch
sudo cp target/release/envswitch /usr/local/bin/

# 一次性配置（自动写入 .zshrc）
envswitch init zsh
exec zsh
```

GUI 构建：

```bash
cd gui
npm install
npx tauri build
open target/release/bundle/macos/envswitch.app
```

依赖：`curl`、`rust`、[Homebrew](https://brew.sh)（php/python/mysql/pgsql 需要）、`fnm`/`nvm`（node 可选）。

### Linux

安装 [Linuxbrew](https://docs.brew.sh/Homebrew-on-Linux)，所有模块和 macOS 完全一致：

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
```

## 支持的模块

| 模块 | 来源 | 类型 | 安装方式 |
|------|------|------|---------|
| **jdk** (8~25) | Azul Zulu API | SDK | 下载 + SHA256 |
| **go** (1.2~1.26) | go.dev API | SDK | 下载 + SHA256 |
| **node** (v0~v24) | nodejs.org API | SDK | 下载 + SHA256 |
| **php** (5.6~8.4) | Homebrew | SDK | `brew install` + shim |
| **python** (3.9~3.14) | Homebrew | SDK | `brew install` + shim |
| **mysql** (8.0~9.x) | Homebrew | 服务 | `brew install` + shim |
| **pgsql** (12~18) | Homebrew | 服务 | `brew install` + shim |

## 使用

```bash
# ── 搜索 ──────────────────────────────────
envswitch search jdk           # Azul API
envswitch search go            # go.dev API
envswitch search node          # nodejs.org API
envswitch search php           # brew search
envswitch search python        # brew search
envswitch search mysql         # brew search
envswitch search pgsql         # brew search

# ── 安装 ─────────────────────────────────
envswitch install jdk 21.0.11
envswitch install go 1.26.3
envswitch install node 24.11.0
envswitch install php 8.3      # brew install php@8.3
envswitch install python 3.14  # brew install python@3.14
envswitch install mysql 8.0    # brew install mysql@8.0
envswitch install pgsql 16     # brew install postgresql@16

# ── 链接（注册已有安装） ──────────────────
envswitch link jdk 8 /Library/Java/JavaVirtualMachines/jdk1.8.0.jdk/Contents/Home

# ── 列表 ─────────────────────────────────
envswitch list                 # 所有模块
envswitch list jdk             # 只看 jdk

# ── 切换 ─────────────────────────────────
envswitch cover jdk 21         # 即时（shim symlink）
envswitch cover go 1.26.3
envswitch cover php 8.3
envswitch cover node 24
envswitch cover python 3.14

# ── 状态 ─────────────────────────────────
envswitch status               # 当前覆盖栈

# ── 取消 ─────────────────────────────────
envswitch uncover jdk          # 恢复系统默认
envswitch uncover --all        # 恢复全部

# ── 服务 ─────────────────────────────────
envswitch start mysql 8.0.46   # 启动 MySQL
envswitch start pgsql 16.14    # 启动 PostgreSQL
envswitch stop mysql           # 停止服务
envswitch stop pgsql
envswitch service-status       # 所有服务状态
envswitch logs mysql --lines 100

# ── 全局 ─────────────────────────────────
envswitch cover jdk 21 --global  # 新终端自动生效

# ── 诊断 ─────────────────────────────────
envswitch doctor               # 检查配置问题

# ── cd 自动切换 ───────────────────────────
envswitch cd-hook on           # 进入 .envswitchrc 目录自动切换

# ── 项目配置 ─────────────────────────────
cd my-project
envswitch init-project         # 创建 .envswitchrc 模板
envswitch auto                 # 应用项目配置
```

## 命令列表

| 命令 | 说明 |
|------|------|
| `search <模块> [-r]` | 搜索可用版本 |
| `install <模块> <版本> [--force]` | 安装版本 |
| `uninstall <模块> <版本> [--purge]` | 卸载版本（--purge 同时删除数据） |
| `link <模块> <版本> <路径>` | 注册已有安装 |
| `list [模块]` | 列出已安装版本 |
| `cover <模块> <版本> [--global]` | 激活版本（即时 shim 切换） |
| `uncover <模块>` | 取消激活 |
| `uncover --all` | 取消全部 |
| `status` | 查看当前覆盖栈 |
| `start <模块> <版本>` | 启动服务 |
| `stop <模块>` | 停止服务 |
| `service-status` | 服务状态 |
| `logs <模块> [--lines N]` | 服务日志 |
| `init [zsh\|bash]` | 配置 shell 集成 |
| `doctor` | 诊断配置问题 |
| `cd-hook <on\|off>` | cd 自动切换开关 |
| `init-project` | 创建 `.envswitchrc` |
| `auto` | 应用 `.envswitchrc` |

## 工作原理

envSwitch 使用 **shims 目录**（`~/.envswitch/shims/`），在 `init` 时一次性加入 PATH。执行 `cover` 时，在 shims 中创建指向目标版本二进制的符号链接。不需要 `eval`，纯文件系统操作，所有终端即时生效。

GUI 使用相同机制 — 点击 Cover 更新 symlinks，终端自动感知变化。

```
~/.envswitch/
├── shims/          # 指向当前版本的 symlinks（已加入 PATH）
├── envs/           # 已安装版本
│   ├── jdk/21.0.11/
│   ├── php/8.3.31/
│   └── mysql/8.0.46/
├── data/           # 服务数据（按版本隔离）
├── state/          # 覆盖栈持久化
├── cache/          # 下载与元数据缓存
├── logs/           # 操作日志
├── config/         # cd-hook、代理配置
├── config.json     # 代理与设置
├── init.sh         # Shell 集成脚本（自动生成）
└── tmp/            # 原子化安装暂存目录
```

## GUI

基于 **Tauri v2 + React + TypeScript + Tailwind CSS**：

```bash
cd gui
npm install
npx tauri dev     # 开发模式（热加载）
npx tauri build   # 生产构建 → .app / .dmg
```

### 功能

- **模块管理** — 列表、搜索、安装、卸载、覆盖、取消覆盖所有模块
- **服务管理** — 启动/停止 MySQL、PostgreSQL，带转圈反馈
- **实时安装日志** — 流式显示 curl 进度条、brew 输出
- **取消并清理** — 三层中断（令牌 + 进程终止 + 暂存回滚）
- **操作日志** — 带时间戳的操作历史（安装、覆盖、启停）
- **自动同步** — 自动检测 Homebrew/系统已安装的 JDK、Go、Node、Python
- **代理支持** — HTTP/HTTPS 代理应用于所有 curl 和 brew 命令
- **中英文切换** — 完整国际化，100+ 翻译键

### 页面

- **版本** — 模块列表，展开卡片显示已安装和可用版本
- **服务** — MySQL/PostgreSQL 卡片，含状态、元数据、启停按钮
- **状态** — 环境覆盖栈总览
- **日志** — 全局操作日志，按级别筛选（OK/INFO/WARN/ERR）
- **诊断** — 诊断检查（平台、brew、模块、shims）
- **设置** — 语言切换、代理配置、CLI 示例

## 代理

为下载设置代理（curl、brew）：

```bash
# CLI：写入配置文件
echo '{"proxy": "http://127.0.0.1:7890"}' > ~/.envswitch/config.json

# GUI：设置 → 代理 → 保存
```

代理应用于所有网络操作：JDK/Go/Node 下载、Homebrew 搜索/安装、版本 API 请求。

## 开发

```bash
# CLI
cargo build -p envswitch --bin envswitch --release
cargo test -- --test-threads=1

# GUI
cd gui && npx tauri dev

# 项目结构
.
├── Cargo.toml          # 工作区根（CLI + 库）
├── src/                # envswitch 库 + CLI 二进制
│   ├── infra/          # 文件系统、下载、操作日志
│   ├── providers/      # jdk、go、node、php、python、mysql、pgsql、homebrew
│   └── config.rs       # 代理与设置
└── gui/
    ├── src/            # React 前端（页面、组件、国际化）
    └── src-tauri/      # Tauri 后端（命令、任务系统）
```

## 许可证

MIT
