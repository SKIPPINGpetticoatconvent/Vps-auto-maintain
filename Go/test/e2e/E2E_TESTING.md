# VPS Telegram Bot E2E 测试指南

本文档介绍如何使用端到端测试验证 Telegram Bot 的按钮交互和 VPS 脚本执行。

## 概述

E2E 测试模拟真实用户与 Telegram Bot 的交互，验证：
- 所有按钮点击是否正确响应
- 维护脚本是否正确执行
- 调度任务是否正确设置
- 权限控制是否有效

## 测试方式

### 1. Go 单元测试（快速）

直接运行 Go 测试，使用模拟对象：

```bash
cd Go
go test -v -run TestE2E_ ./test/e2e/... -count=1
```

### 2. Podman 容器测试（推荐）

使用 Podman 在真实 Debian 环境中测试：

```bash
cd Go/test/e2e
chmod +x run_podman_e2e.sh
./run_podman_e2e.sh
```

#### Podman 测试选项

```bash
# 完整测试（Go 测试 + 容器测试）
./run_podman_e2e.sh

# 仅运行 Go 测试
./run_podman_e2e.sh --go-tests

# 仅构建镜像
./run_podman_e2e.sh --build-only

# 仅运行容器测试
./run_podman_e2e.sh --run-only

# 清理资源
./run_podman_e2e.sh --cleanup
```

### 3. Podman Compose（多服务测试）

```bash
cd Go/test/e2e

# 启动 Bot 服务
podman-compose -f podman-compose.yml up -d vps-bot

# 运行脚本验证
podman-compose -f podman-compose.yml --profile validate up script-validator

# 运行完整 E2E 测试
podman-compose -f podman-compose.yml --profile test up e2e-tester

# 停止所有服务
podman-compose -f podman-compose.yml down
```

## 测试覆盖

### 测试用例列表

| 测试名称 | 描述 | 验证内容 |
|---------|------|---------|
| `TestE2E_StartCommand` | /start 命令 | 主菜单显示 |
| `TestE2E_MainMenuNavigation` | 主菜单导航 | 所有按钮响应 |
| `TestE2E_MaintenanceFlow` | 维护流程 | 脚本执行 |
| `TestE2E_ScheduleFlow` | 调度设置 | Cron 任务 |
| `TestE2E_MultiLevelMenuFlow` | 多级菜单 | 菜单层级 |
| `TestE2E_ViewTasksList` | 任务列表 | 数据展示 |
| `TestE2E_UnauthorizedAccess` | 权限控制 | 拒绝未授权 |
| `TestE2E_RebootConfirm` | 重启确认 | 安全确认 |
| `TestE2E_StatusCheck` | 系统状态 | 状态获取 |
| `TestE2E_ConcurrentButtonClicks` | 并发点击 | 线程安全 |
| `TestE2E_RapidButtonClicks` | 快速点击 | 防抖处理 |
| `TestE2E_FullUserJourney` | 完整旅程 | 端到端流程 |
| `TestE2E_InvalidCallbackData` | 无效回调 | 错误处理 |
| `TestE2E_FrequencySelectionMenu` | 频率选择 | 菜单交互 |
| `TestE2E_TimeSelectionMenu` | 时间选择 | 时间设置 |
| `TestE2E_BackNavigation` | 返回导航 | 菜单返回 |

### 脚本验证

容器测试验证以下脚本：
- `/usr/local/bin/vps-maintain-core.sh` - 核心维护
- `/usr/local/bin/vps-maintain-rules.sh` - 规则更新
- `/usr/local/bin/x-ui restart` - Xray 重启
- `/usr/local/bin/sb restart` - Sing-box 重启

## 环境变量

| 变量 | 描述 | 默认值 |
|-----|------|-------|
| `TG_TOKEN` | Telegram Bot Token | 测试 Token |
| `TG_CHAT_ID` | 管理员 Chat ID | 123456789 |
| `TEST_MODE` | 测试模式标志 | true |

## 文件结构

```
Go/test/e2e/
├── bot_e2e_test.go      # E2E 测试代码
├── Dockerfile.e2e       # E2E 测试镜像
├── podman-compose.yml   # Podman Compose 配置
├── run_e2e_tests.sh     # 简单测试脚本
├── run_podman_e2e.sh    # Podman 测试脚本
└── E2E_TESTING.md       # 本文档
```

## 故障排查

### 测试失败

1. 检查依赖：
   ```bash
   go mod tidy
   ```

2. 检查 Podman：
   ```bash
   podman --version
   podman info
   ```

3. 查看容器日志：
   ```bash
   podman logs vps-tg-bot-e2e
   ```

### 构建失败

1. 确保 Go 版本 >= 1.21
2. 检查网络连接（下载依赖）
3. 检查磁盘空间

## 持续集成

GitHub Actions 示例：

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Go
        uses: actions/setup-go@v4
        with:
          go-version: '1.21'
      
      - name: Run E2E Tests
        run: |
          cd Go
          go test -v -run TestE2E_ ./test/e2e/... -count=1
```
