# VPS Telegram Bot - 完整 E2E 测试指南

本文档详细介绍如何使用 Podman 进行端到端测试，包括性能测试、安全性测试和 CI/CD 集成。

## 概述

本项目提供三层测试架构：

1. **本地 Mock 测试** - 快速验证业务逻辑
2. **容器化 E2E 测试** - 模拟真实 VPS 环境
3. **多发行版兼容性测试** - 验证跨平台兼容性

## 项目结构

```
VPS-Auto-Maintain/
├── Go/
│   ├── .github/workflows/go-e2e-tests.yml     # Go CI/CD 配置
│   ├── cmd/vps-tg-bot/                        # Go 源代码
│   └── test/
│       ├── e2e/
│       │   ├── bot_e2e_test.go                # 基础 E2E 测试
│       │   ├── performance_test.go            # 性能测试
│       │   ├── security_test.go               # 安全测试
│       │   ├── Dockerfile.e2e                 # Go E2E 测试镜像
│       │   ├── podman-compose.yml             # Podman Compose 配置
│       │   └── run_podman_e2e.sh              # Go Podman 测试脚本
│       └── container/
│           ├── Dockerfile.debian              # Debian 测试镜像
│           ├── Dockerfile.ubuntu              # Ubuntu 测试镜像
│           ├── Dockerfile.centos              # CentOS 测试镜像
│           └── podman-compose.yml             # 多发行版测试配置
├── Rust/
│   ├── .github/workflows/rust-e2e-tests.yml   # Rust CI/CD 配置
│   ├── src/                                   # Rust 源代码
│   └── tests/
│       ├── e2e_test.rs                        # 基础 E2E 测试
│       ├── integration_test.rs                # 集成测试
│       ├── scheduler_test.rs                  # 调度器测试
│       └── e2e/
│           ├── e2e_test.rs                    # Rust E2E 测试
│           ├── performance_test.rs            # Rust 性能测试
│           ├── security_test.rs               # Rust 安全测试
│           ├── Dockerfile.e2e                 # Rust E2E 测试镜像
│           ├── podman-compose.yml             # Rust Podman Compose
│           └── run_podman_e2e.sh              # Rust Podman 测试脚本
└── README.md                                  # 项目说明
```

## 测试类型

### 1. 基础 E2E 测试

验证 Bot 的基本功能：
- 命令处理 (`/start`)
- 菜单导航
- 维护操作
- 调度设置
- 权限控制
- 多级菜单交互

### 2. 性能测试

测试系统在高负载下的表现：
- 基本响应时间测试
- 并发请求处理
- 内存使用监控
- 高频点击处理
- 大消息处理
- 压力测试

### 3. 安全测试

验证系统安全性：
- 命令注入防护
- 路径遍历攻击防护
- XSS 攻击防护
- 恶意输入处理
- 缓冲区溢出防护
- 资源耗尽攻击防护
- Unicode 字符安全处理

### 4. 集成测试

测试组件间集成：
- Bot 与系统集成
- 调度器与系统集成
- 状态持久化
- 错误处理
- 并发安全

## 本地测试

### Go 测试

```bash
# 运行所有测试
cd Go
go test -v ./...

# 运行特定测试
go test -v -run TestE2E_ ./test/e2e/...
go test -v -run TestPerformance_ ./test/e2e/...
go test -v -run TestSecurity_ ./test/e2e/...

# 运行带覆盖率的测试
go test -v -coverprofile=coverage.out ./...
go tool cover -html=coverage.out -o coverage.html
```

### Rust 测试

```bash
# 运行所有测试
cd Rust/vps-tg-bot
cargo test --release --verbose

# 运行特定测试
cargo test e2e_test --release --verbose
cargo test performance_test --release --verbose
cargo test security_test --release --verbose

# 运行带文档的测试
cargo test --doc --release --verbose

# 代码质量检查
cargo fmt --check
cargo clippy --all-targets --all-features
cargo audit
```

## Podman 容器测试

### Go Podman 测试

```bash
cd Go/test/e2e

# 完整测试流程
chmod +x run_podman_e2e.sh
./run_podman_e2e.sh

# 仅运行 Go 单元测试
./run_podman_e2e.sh --go-tests

# 仅构建镜像
./run_podman_e2e.sh --build-only

# 仅运行容器测试
./run_podman_e2e.sh --run-only

# 清理资源
./run_podman_e2e.sh --cleanup
```

### Rust Podman 测试

```bash
cd Rust/vps-tg-bot/tests/e2e

# 完整测试流程
chmod +x run_podman_e2e.sh
./run_podman_e2e.sh

# 仅运行 Rust 单元测试
./run_podman_e2e.sh --rust-tests

# 仅构建镜像
./run_podman_e2e.sh --build-only

# 仅运行容器测试
./run_podman_e2e.sh --run-only

# 清理资源
./run_podman_e2e.sh --cleanup
```

## 多发行版测试

### 使用 Podman Compose

```bash
# 启动 Bot 服务
podman-compose -f podman-compose.yml up -d vps-bot

# 运行脚本验证
podman-compose -f podman-compose.yml --profile validate up script-validator

# 运行完整 E2E 测试
podman-compose -f podman-compose.yml --profile test up e2e-tester

# 停止所有服务
podman-compose -f podman-compose.yml down
```

### 多发行版兼容性测试

```bash
cd Go/test/container

# 测试所有发行版
chmod +x run_multi_distro_tests.sh
./run_multi_distro_tests.sh

# 测试特定发行版
./run_multi_distro_tests.sh debian
./run_multi_distro_tests.sh ubuntu
./run_multi_distro_tests.sh centos
```

## 测试指标

### 性能指标

| 指标 | Go 期望值 | Rust 期望值 | 测试方法 |
|------|-----------|-------------|----------|
| 平均响应时间 | < 100ms | < 50ms | `TestPerformance_BasicResponseTime` |
| 并发处理能力 | > 100 QPS | > 200 QPS | `TestPerformance_ConcurrentRequests` |
| 内存增长 | < 1KB/操作 | < 512B/操作 | `TestPerformance_MemoryUsage` |
| 错误率 | < 5% | < 2% | 高频点击测试 |

### 安全指标

| 安全测试 | 覆盖率 | 说明 |
|----------|--------|------|
| 命令注入防护 | 100% | 阻止所有已知的注入模式 |
| XSS 防护 | 100% | HTML 转义和输入验证 |
| 路径遍历防护 | 100% | 文件路径验证 |
| 恶意输入处理 | 95%+ | 超长输入、特殊字符处理 |
| 资源耗尽防护 | 90%+ | 并发请求限制和超时 |

### 测试覆盖率

- **Go**: > 85% 代码覆盖率
- **Rust**: > 90% 代码覆盖率
- **E2E 场景**: 100% 主要用户路径覆盖

## CI/CD 集成

### GitHub Actions

项目配置了完整的 CI/CD 流水线：

#### Go 工作流
```yaml
# 触发条件
- push 到 main/develop 分支
- pull request 到 main 分支
- 路径变更包含 Go 相关文件

# 工作流程
1. Go 单元测试 (多版本)
2. Go 集成测试
3. Podman E2E 测试
4. 多发行版兼容性测试
5. 安全测试 (gosec, trivago)
6. 性能基准测试
7. 构建和多平台打包
8. 测试报告生成
```

#### Rust 工作流
```yaml
# 触发条件
- push 到 main/develop 分支
- pull request 到 main 分支
- 路径变更包含 Rust 相关文件

# 工作流程
1. Rust 单元测试 (多版本)
2. Rust 集成测试
3. Podman E2E 测试
4. 跨平台构建测试
5. 安全测试 (cargo-audit, cargo-deny)
6. 性能基准测试
7. 构建和多平台打包
8. 测试报告生成
```

### 测试报告

CI/CD 流水线会自动生成测试报告，包括：
- 测试执行结果
- 代码覆盖率报告
- 性能基准对比
- 安全扫描结果
- 构建工件下载链接

## 环境变量

### 通用变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `TEST_MODE` | 测试模式标志 | `true` |
| `TG_TOKEN` | Telegram Bot Token | 测试 Token |
| `TG_CHAT_ID` | 管理员 Chat ID | `123456789` |

### Go 专用变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `STATE_FILE` | 状态文件路径 | `./state.json` |
| `CORE_SCRIPT` | 核心维护脚本路径 | `./vps-maintain-core.sh` |
| `RULES_SCRIPT` | 规则维护脚本路径 | `./vps-maintain-rules.sh` |
| `HISTORY_FILE` | 历史记录文件路径 | `./maintain_history.json` |

### Rust 专用变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `TELOXIDE_TOKEN` | Telegram Bot Token | 测试 Token |
| `CHAT_ID` | 管理员 Chat ID | `123456789` |
| `RUST_LOG` | 日志级别 | `info` |

## 故障排除

### 常见问题

#### 1. Podman 权限问题
```bash
# 检查 Podman 版本
podman --version

# 使用特权模式运行容器
podman run --privileged ...

# 或者添加网络管理权限
podman run --cap-add=NET_ADMIN ...
```

#### 2. 镜像构建失败
```bash
# 清理 Docker/Podman 缓存
podman system prune -a

# 重新构建镜像
podman build --no-cache -t vps-tg-bot-e2e .
```

#### 3. 测试超时
```bash
# 增加测试超时时间
go test -timeout=10m ./...
cargo test -- --test-timeout=600s
```

#### 4. 网络连接问题
```bash
# 检查网络连接
podman network ls
podman network inspect e2e-network

# 重新创建网络
podman network rm e2e-network
podman network create e2e-network
```

### 调试技巧

#### 1. 查看容器日志
```bash
# Go 容器日志
podman logs vps-tg-bot-e2e-test

# Rust 容器日志
podman logs vps-tg-bot-rust-e2e-test

# 实时跟踪日志
podman logs -f vps-tg-bot-e2e-test
```

#### 2. 进入容器调试
```bash
# 进入正在运行的容器
podman exec -it vps-tg-bot-e2e-test /bin/bash

# 以新容器方式进入
podman run -it --rm vps-tg-bot-e2e /bin/bash
```

#### 3. 性能分析
```bash
# Go 性能分析
go test -bench=. -benchmem ./...
go test -v -race ./...

# Rust 性能分析
cargo bench --verbose
cargo test performance_test --release --verbose
```

## 最佳实践

### 1. 测试开发
- 为新功能编写对应的测试用例
- 确保测试覆盖所有边界情况
- 使用有意义的测试名称和描述
- 定期更新测试数据

### 2. 性能优化
- 监控内存使用情况
- 避免不必要的对象创建
- 使用连接池复用资源
- 实现适当的超时机制

### 3. 安全考虑
- 验证所有用户输入
- 使用参数化查询
- 实施速率限制
- 定期更新依赖项

### 4. 持续集成
- 在每次提交时运行测试
- 设置质量门禁
- 生成详细的测试报告
- 自动发布构建工件

## 贡献指南

### 提交测试
1. Fork 项目仓库
2. 创建功能分支
3. 编写测试用例
4. 运行完整测试套件
5. 提交 Pull Request

### 测试标准
- 所有新功能必须包含测试
- 测试覆盖率不能降低
- 性能测试必须通过基准
- 安全测试必须全部通过

## 参考资源

- [Go 测试指南](https://golang.org/doc/tutorial/add-a-test)
- [Rust 测试指南](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Podman 文档](https://docs.podman.io/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)

## 许可证

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。