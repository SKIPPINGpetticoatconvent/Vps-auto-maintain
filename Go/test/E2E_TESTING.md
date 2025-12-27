# VPS Telegram Bot E2E 测试指南

## 测试架构

本项目包含三层测试：

### 1. 本地 Mock 测试 (Windows/Linux 均可运行)
- **Go**: `Go/test/integration/integration_test.go`
- **Rust**: `Rust/vps-tg-bot/tests/integration_test.rs`

适用于：
- 纯逻辑函数验证
- JSON/配置解析
- 消息格式化
- Cron 表达式计算

运行命令：
```bash
# Go
cd Go && go test -v ./test/integration/...

# Rust
cd Rust/vps-tg-bot && cargo test --test integration_test
```

### 2. 容器化 E2E 测试 (需要 Podman)
- **位置**: `Go/test/container/`

适用于：
- Linux 系统命令调用 (systemctl, apt, ufw)
- 权限与文件系统测试
- 网络防火墙逻辑
- Fail2Ban 联动测试
- 服务管理测试

运行命令：
```bash
cd Go/test/container
chmod +x run_e2e_tests.sh
./run_e2e_tests.sh all
```

### 3. 多发行版兼容性测试
- **位置**: `Go/test/container/`
- **支持**: Debian, Ubuntu, CentOS

运行命令：
```bash
cd Go/test/container
chmod +x run_multi_distro_tests.sh
./run_multi_distro_tests.sh
```

## 测试文件结构

```
Go/test/
├── integration/
│   └── integration_test.go      # 本地集成测试
└── container/
    ├── Dockerfile.e2e           # 主 E2E 测试镜像
    ├── Dockerfile.debian        # Debian 测试镜像
    ├── Dockerfile.ubuntu        # Ubuntu 测试镜像
    ├── Dockerfile.centos        # CentOS 测试镜像
    ├── podman-compose.yml       # 多容器编排
    ├── run_e2e_tests.sh         # E2E 测试脚本
    └── run_multi_distro_tests.sh # 多发行版测试脚本

Rust/vps-tg-bot/tests/
├── integration_test.rs          # 本地集成测试
└── e2e_test.rs                  # E2E 测试
```

## 测试覆盖范围

### 本地测试 (13 Go + 12 Rust = 25 个测试用例)
| 测试项 | Go | Rust |
|--------|:--:|:----:|
| 配置加载 | ✅ | ✅ |
| Bot 到系统集成 | ✅ | ✅ |
| Bot 到调度器集成 | ✅ | ✅ |
| 调度器到系统集成 | ✅ | - |
| 维护工作流 | ✅ | ✅ |
| 调度工作流 | ✅ | ✅ |
| 状态持久化 | ✅ | ✅ |
| 并发请求 | ✅ | ✅ |
| 错误处理 | ✅ | ✅ |
| 授权验证 | ✅ | ✅ |
| 多级菜单 | ✅ | - |
| 服务重启 | ✅ | - |
| 更新操作 | ✅ | ✅ |
| 未知回调 | - | ✅ |
| 消息格式 | - | ✅ |

### 容器化测试
| 测试项 | 描述 |
|--------|------|
| 系统命令 | systemctl, apt-get, ufw, iptables |
| 权限测试 | /etc/systemd/system/, /tmp/, /var/lib/ |
| 防火墙 | iptables 规则, ufw 状态 |
| Fail2Ban | 配置验证, SSH 封禁模拟 |
| 服务管理 | x-ui, sing-box 命令 |

### 多发行版测试
| 发行版 | 包管理器 | 防火墙 |
|--------|----------|--------|
| Debian Bookworm | apt | ufw/iptables |
| Ubuntu 22.04 | apt | ufw/iptables |
| CentOS Stream 9 | dnf | firewalld |

## 前置要求

### 本地测试
- Go 1.21+
- Rust 1.70+

### 容器化测试
- Podman 4.0+
- podman-compose (可选)

## CI/CD 集成

```yaml
# GitHub Actions 示例
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      # 本地测试
      - name: Go Tests
        run: cd Go && go test -v ./test/integration/...
      
      - name: Rust Tests
        run: cd Rust/vps-tg-bot && cargo test --test integration_test
      
      # 容器化测试
      - name: Container E2E Tests
        run: |
          cd Go/test/container
          chmod +x run_e2e_tests.sh
          ./run_e2e_tests.sh all
```

## 故障排除

### Podman 权限问题
```bash
# 使用 --privileged 运行容器
podman run --privileged ...
```

### iptables 不可用
```bash
# 确保容器以特权模式运行
# 或使用 --cap-add=NET_ADMIN
```

### Fail2Ban 服务未启动
```bash
# 容器内手动启动
service fail2ban start
```
