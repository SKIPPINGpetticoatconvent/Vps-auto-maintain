# VPS Telegram Bot - Go 版本

基于 Go 语言实现的 VPS Telegram Bot 管理系统，从 Python 版本迁移而来。

## 功能特性

- ✅ 系统状态查询 (`/start` → 📊 系统状态)
- ✅ 立即执行维护 (`/start` → 🔧 立即维护)
- ✅ 查看服务日志 (`/start` → 📋 查看日志)
- ✅ 重启 VPS (`/start` → ♻️ 重启 VPS)
- ✅ 每周日 04:00 自动维护（系统更新 + 规则更新 + 自动重启）

## 项目结构

```
Go/
├── cmd/
│   └── vps-tg-bot/
│       └── main.go          # 主程序入口
├── pkg/
│   ├── config/
│   │   └── config.go        # 配置加载模块
│   ├── bot/
│   │   └── handler.go       # Bot 核心逻辑和指令处理器
│   ├── system/
│   │   └── actions.go       # 系统操作函数
│   └── scheduler/
│       └── scheduler.go     # 定时任务调度器
├── go.mod                   # Go 模块定义
├── go.sum                   # 依赖校验和
├── vps-tg-bot-install.sh    # 一键部署脚本
└── README.md                # 本文档
```

## 快速开始

### 1. 部署

在 Linux 服务器上以 root 权限运行：

```bash
cd Go
chmod +x vps-tg-bot-install.sh
./vps-tg-bot-install.sh
```

部署脚本会自动：
1. 检查并安装 Go 环境
2. 同步 VPS 时区
3. 配置 journald 内存日志
4. 创建维护脚本
5. 编译 Go 程序
6. 创建并启动 systemd 服务

### 2. 配置

程序支持两种配置方式：

#### 方式一：环境变量（推荐）

```bash
export TG_TOKEN="your_bot_token"
export TG_CHAT_ID="your_chat_id"
./vps-tg-bot
```

#### 方式二：交互式输入

如果环境变量未设置，程序会自动提示输入配置信息：

```bash
./vps-tg-bot
```

程序会依次提示：
1. Telegram Bot Token（隐藏输入，从 [@BotFather](https://t.me/BotFather) 获取）
2. Telegram Chat ID（管理员，从 [@userinfobot](https://t.me/userinfobot) 获取）
3. 核心维护脚本路径（可选，有默认值）
4. 规则更新脚本路径（可选，有默认值）

详细配置说明请查看 [CONFIG.md](CONFIG.md)

### 3. 使用

在 Telegram 中向 Bot 发送 `/start` 命令，即可打开管理面板。

## 手动编译

如果需要手动编译：

```bash
cd Go
go mod download
go build -o vps-tg-bot ./cmd/vps-tg-bot
```

## 环境变量

程序优先从环境变量读取配置，如果未设置且处于交互式终端，会提示用户输入：

- `TG_TOKEN`: Telegram Bot Token（必需）
- `TG_CHAT_ID`: 管理员 Chat ID（必需）
- `CORE_SCRIPT`: 核心维护脚本路径（默认: `/usr/local/bin/vps-maintain-core.sh`）
- `RULES_SCRIPT`: 规则更新脚本路径（默认: `/usr/local/bin/vps-maintain-rules.sh`）

**注意**：在 systemd 服务中运行时，必须使用环境变量配置，因为服务不是交互式终端。

## 系统服务管理

```bash
# 查看服务状态
systemctl status vps-tg-bot

# 查看日志
journalctl -u vps-tg-bot -f

# 重启服务
systemctl restart vps-tg-bot

# 停止服务
systemctl stop vps-tg-bot

# 启动服务
systemctl start vps-tg-bot
```

## 定时任务

系统会在每周日 04:00 自动执行：
1. Xray 规则文件更新
2. 系统更新（apt-get update/upgrade）
3. Xray/Sing-box 更新
4. 自动重启 VPS

## 维护脚本

系统使用以下维护脚本：

- `/usr/local/bin/vps-maintain-core.sh`: 核心维护（系统更新、Xray/Sing-box 更新）
- `/usr/local/bin/vps-maintain-rules.sh`: 规则文件更新

## 开发

### 依赖

- Go 1.19+
- `github.com/go-telegram-bot-api/telegram-bot-api/v5`
- `github.com/robfig/cron/v3`

### 使用 Makefile（推荐）

```bash
cd Go
make help        # 查看所有可用命令
make build       # 构建二进制文件
make test        # 运行测试
make lint        # 代码检查
make fmt         # 格式化代码
make build-all   # 构建所有平台
```

### 本地测试

```bash
export TG_TOKEN="your_bot_token"
export TG_CHAT_ID="your_chat_id"
go run ./cmd/vps-tg-bot
```

或使用 Makefile：
```bash
make run
```

## CI/CD

项目使用 GitHub Actions 进行自动构建和发布。详细说明请查看 [CI.md](CI.md)。

### 快速开始

1. **推送代码**：推送到 `main` 或 `master` 分支会自动触发构建
2. **创建 Release**：打标签 `v1.0.0` 会自动创建 Release 并上传构建产物
3. **查看构建**：在 GitHub 仓库的 "Actions" 标签页查看构建状态

### 工作流

- `go-build.yml` - 完整构建（多平台 + Release）
- `go-build-simple.yml` - 简化构建（快速验证）
- `go-lint.yml` - 代码质量检查

## 与 Python 版本的对比

| 特性 | Python 版本 | Go 版本 |
|------|------------|---------|
| 运行时 | Python 3 + 虚拟环境 | 单一二进制文件 |
| 依赖管理 | uv/pip | go mod |
| 启动速度 | 较慢 | 快速 |
| 资源占用 | 较高 | 较低 |
| 部署复杂度 | 需要 Python 环境 | 仅需 Go 编译器 |

## 许可证

与主项目保持一致。

## 更新日志

### v1.0.0
- 初始版本
- 从 Python 版本迁移到 Go
- 实现所有核心功能
- 支持定时任务调度
