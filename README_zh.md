# envSwitch

轻量级开发环境版本管理器，支持 macOS 和 Linux。

一键切换 JDK、Go、PHP、Python、MySQL、PostgreSQL 版本 — 无需 Docker，无需编译，不污染 shell 配置。

```bash
$ envswitch search jdk
  21.0.11+10.0.LTS
  17.0.19+10
  8.0.492

$ envswitch install jdk 21.0.11+10.0.LTS
# 从 Azul Zulu API 下载，SHA256 校验

$ envswitch cover jdk 21.0.11+10.0.LTS
$ java -version  # JDK 21 ✓

$ envswitch uncover jdk
$ java -version  # 恢复系统默认
```

## 为什么用 envSwitch？

| 特性 | envSwitch | Docker | NVM/pyenv | ServBay |
|------|-----------|--------|-----------|---------|
| **速度** | 原生二进制 | 虚拟机开销 | Shell 脚本 | 原生 |
| **安装** | `brew install` | `docker pull` | 脚本 | GUI |
| **切换** | 即时（symlinks） | 重启容器 | Shell eval | 即时 |
| **新终端** | 自动生效 | 手动 | 配置 rc | 手动 GUI |
| **服务管理** | ✓ (MySQL, PgSQL) | ✓ | ✗ | ✓ |
| **数据隔离** | 按版本隔离 | 容器级 | 全局 | 全局 |
| **体积** | ~3MB 二进制 | GB 级 | ~50MB | ~200MB |

## 安装

```bash
git clone https://github.com/your/envswitch.git
cd envswitch
cargo build --release
sudo cp target/release/envswitch /usr/local/bin/

# 一次性配置
envswitch init zsh
exec zsh
```

依赖：[Homebrew](https://brew.sh)（PHP、Python、MySQL、PostgreSQL 需要）、`curl`。

## 支持的模块

| 模块 | 来源 | 类型 |
|------|------|------|
| **jdk** | Azul Zulu API | SDK |
| **go** | go.dev API | SDK |
| **php** | Homebrew | SDK |
| **python** | Homebrew | SDK |
| **mysql** | Homebrew | 服务 |
| **pgsql** | Homebrew | 服务 |

## 使用

```bash
# 搜索可用版本
envswitch search <模块名>

# 安装版本
envswitch install <模块名> <版本号>

# 列出已安装版本
envswitch list
envswitch list jdk

# 切换版本（即时生效）
envswitch cover jdk 21
envswitch cover go 1.25.10
envswitch cover php 8.3
envswitch cover python 3.14

# 查看当前覆盖状态
envswitch status

# 取消覆盖（恢复系统默认）
envswitch uncover jdk
envswitch uncover --all

# 服务管理（MySQL、PostgreSQL）
envswitch start mysql 8.0.46
envswitch stop mysql
envswitch logs mysql --lines 100

# 全局覆盖（新终端自动生效）
envswitch cover jdk 21 --global

# 项目配置
cd my-project
envswitch init-project
# 编辑 .envswitchrc，然后：
envswitch auto
```

## 工作原理

envSwitch 使用 **shims 目录**（`~/.envswitch/shims/`），在 `init` 时一次性添加到 PATH 中。执行 `cover` 时，envSwitch 在 shims 目录中创建指向目标版本二进制的符号链接（symlink）。不需要 `eval`，不需要 shell hack — 纯文件系统操作，所有终端即时生效。

```
~/.envswitch/
├── shims/          # 指向当前激活版本的 symlinks（已加入 PATH）
├── envs/           # 已安装的版本
│   ├── jdk/21.0.11/
│   ├── php/8.3.31/
│   └── mysql/8.0.46/
├── data/           # 服务数据（按版本隔离）
│   ├── mysql/8.0.46/
│   └── pgsql/16.14/
├── state/          # 覆盖栈持久化
├── cache/          # 下载缓存
└── init.sh         # Shell 集成脚本（自动生成）
```

## 命令列表

| 命令 | 说明 |
|------|------|
| `search <模块>` | 搜索可用版本 |
| `install <模块> <版本>` | 安装版本 |
| `uninstall <模块> <版本>` | 卸载版本 |
| `list [模块]` | 列出已安装版本 |
| `cover <模块> <版本>` | 激活版本 |
| `uncover <模块>` | 取消激活 |
| `uncover --all` | 取消所有激活 |
| `status` | 查看当前覆盖栈 |
| `start <模块> <版本>` | 启动服务 |
| `stop <模块>` | 停止服务 |
| `service-status` | 查看服务状态 |
| `logs <模块>` | 查看服务日志 |
| `init` | 配置 shell 集成 |
| `init-project` | 创建 `.envswitchrc` 模板 |
| `auto` | 应用 `.envswitchrc` 配置 |

## 开发

```bash
cargo build --release
cargo test -- --test-threads=1
```

## 许可证

MIT
