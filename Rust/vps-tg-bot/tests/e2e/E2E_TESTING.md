# Rust VPS Telegram Bot E2E 测试指南

本文档介绍如何使用 Podman 进行端到端测试，验证 Telegram Bot 的按钮交互和 VPS 脚本执行。

## 概述

E2E 测试模拟真实用户与 Telegram Bot 的交互，验证：
- 所有按钮点击是否正确响应
- 维护脚本是否正确执行
- 调度任务是否正确设置
- 权限控制是否有效

## 测试方式

### 1. Rust 单元测试（快速）

直接运行 Rust 测试，使用模拟对象：

```bash
cd Rust/vps-tg-bot
cargo test e2e_test --release -- --test-threads=1 --nocapture
```

### 2. Podman 容器测试（推荐）

使用 Podman 在真实 Debian 环境中测试：

```bash
cd Rust/vps-tg-bot/tests/e2e
chmod +x run_podman_e2e.sh
./run_podman_e2e.sh
```

#### Podman 测试选项

```bash
# 完整测试（Rust 测试 + 容器测试）
./run_podman_e2e.sh

# 仅运行 Rust 测试
./run_podman_e2e.sh --rust-tests

# 仅构建镜像
./run_podman_e2e.sh --build-only

# 仅运行容器测试
./run_podman_e2e.sh --run-only

# 清理资源
./run_podman_e2e.sh --cleanup
```

### 3. Podman Compose（多服务测试）

```bash
cd Rust/vps-tg-bot/tests/e2e

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
| `test_start_command` | /start 命令 | 主菜单显示 |
| `test_main_menu_buttons` | 主菜单按钮 | 状态/维护/调度/日志 |
| `test_maintain_menu_buttons` | 维护菜单按钮 | 核心/规则/Xray/Sing-box |
| `test_schedule_menu_buttons` | 调度菜单按钮 | 任务类型选择 |
| `test_back_navigation` | 返回导航 | 菜单返回 |
| `test_unauthorized_access` | 权限控制 | 拒绝未授权 |
| `test_preset_buttons` | 预设按钮 | 每日/每周/每月 |
| `test_time_selection` | 时间选择 | 具体时间设置 |
| `test_custom_schedule` | 自定义调度 | Cron 表达式 |
| `test_invalid_callback` | 无效回调 | 错误处理 |
| `test_full_user_journey` | 完整旅程 | 端到端流程 |
| `test_concurrent_callbacks` | 并发回调 | 线程安全 |

### 脚本验证

容器测试验证以下脚本：
- `/usr/local/bin/vps-maintain-core.sh` - 核心维护
- `/usr/local/bin/vps-maintain-rules.sh` - 规则更新
- `/usr/local/bin/x-ui {restart|status|update}` - Xray 服务
- `/usr/local/bin/sb {restart|status|update}` - Sing-box 服务

## 环境变量

| 变量 | 描述 | 默认值 |
|-----|------|-------|
| `TELOXIDE_TOKEN` | Telegram Bot Token | 测试 Token |
| `CHAT_ID` | 管理员 Chat ID | 123456789 |
| `RUST_LOG` | 日志级别 | info |

## 文件结构

```
Rust/vps-tg-bot/tests/
├── e2e_test.rs          # E2E 测试代码
├── e2e/
│   ├── Dockerfile.e2e       # Podman 测试镜像
│   ├── podman-compose.yml   # Podman Compose 配置
│   ├── run_podman_e2e.sh    # Podman 测试脚本
│   └── E2E_TESTING.md       # 本文档
└── scheduler_test.rs    # 调度器测试
```

## 故障排查

### 测试失败

1. 检查 Rust 工具链：
   ```bash
   rustup update
   cargo --version
   ```

2. 检查 Podman：
   ```bash
   podman --version
   podman info
   ```

3. 查看容器日志：
   ```bash
   podman logs vps-tg-bot-rust-e2e
   ```

### 构建失败

1. 确保 Rust 版本 >= 1.70
2. 检查依赖：
   ```bash
   cargo check
   ```
3. 安装 OpenSSL 开发包（如需要）：
   ```bash
   # Debian/Ubuntu
   sudo apt-get install libssl-dev pkg-config
   ```

## 持续集成

GitHub Actions 示例：

```yaml
name: Rust E2E Tests

on: [push, pull_request]

jobs:
  e2e-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
      
      - name: Run E2E Tests
        run: |
          cd Rust/vps-tg-bot
          cargo test e2e_test --release -- --test-threads=1
```

## 与 Go 版本对比

| 功能 | Rust 版本 | Go 版本 |
|-----|----------|--------|
| 测试框架 | Cargo Test | Go Test |
| 模拟方式 | MockTelegramBot | MockTelegramAPI |
| 并发测试 | std::thread | sync.WaitGroup |
| 容器基础 | Debian + Rust | Debian + Go |
