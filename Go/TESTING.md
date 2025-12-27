# 容器化测试指南

本指南详细说明了如何使用VPS Telegram Bot项目的容器化测试功能。

## 概述

项目现在支持完整的容器化测试，包括：
- 单元测试和集成测试
- 性能测试
- 安全测试
- 覆盖率报告
- 自动化测试环境

## 快速开始

### 1. 基本测试

```bash
# 运行基本容器化测试
make test-containerized

# 或者使用docker-compose
docker-compose -f docker-compose.test.yml up vps-tg-bot-test
```

### 2. 运行所有测试

```bash
# 运行所有类型的测试
make test-all

# 或者使用docker-compose profiles
docker-compose -f docker-compose.test.yml --profile all up
```

## 详细说明

### 测试类型

#### 1. 容器化测试 (test-containerized)

- 运行所有单元测试和集成测试
- 生成覆盖率报告
- 执行静态代码分析
- 在隔离的容器环境中运行

```bash
# 本地运行
make test-containerized

# 容器运行
docker-compose -f docker-compose.test.yml up vps-tg-bot-test
```

#### 2. 性能测试 (test-performance)

- 内存使用测试
- 并发性能测试
- 响应时间测试
- 基准测试

```bash
# 本地运行
make test-performance

# 容器运行
docker-compose -f docker-compose.test.yml --profile performance up performance-test
```

#### 3. 安全测试 (test-security)

- 输入验证测试
- 权限控制测试
- 安全配置测试
- 静态安全分析
- 依赖漏洞检查

```bash
# 本地运行
make test-security

# 容器运行
docker-compose -f docker-compose.test.yml --profile security up security-test
```

#### 4. 覆盖率报告 (coverage-report)

```bash
# 生成覆盖率报告
make coverage-report
# 报告将生成在 test-results/coverage.html
```

### Docker Compose 配置

#### 标准测试环境

```yaml
services:
  vps-tg-bot-test:
    build:
      context: .
      dockerfile: Dockerfile.test
    environment:
      - TEST_MODE=true
      - TG_TOKEN=test_token_123456789:test_bot_token_for_testing
      - TG_CHAT_ID=123456789
    volumes:
      - ./test-results:/app/test-results
```

#### 特殊测试环境

```bash
# 覆盖率收集
docker-compose -f docker-compose.test.yml --profile coverage up coverage-collector

# 性能测试
docker-compose -f docker-compose.test.yml --profile performance up performance-test

# 安全测试
docker-compose -f docker-compose.test.yml --profile security up security-test
```

### 环境变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `TEST_MODE` | 启用测试模式 | `true` |
| `TG_TOKEN` | 测试用Telegram机器人令牌 | `test_token_123456789:test_bot_token_for_testing` |
| `TG_CHAT_ID` | 测试用聊天ID | `123456789` |
| `STATE_FILE` | 测试状态文件路径 | `/app/test-results/test_state.json` |
| `COVERAGE_DIR` | 覆盖率报告目录 | `/app/test-results` |
| `TEST_TIMEOUT` | 测试超时时间 | `30s` |
| `PERFORMANCE_TEST` | 启用性能测试 | `true` |
| `SECURITY_TEST` | 启用安全测试 | `true` |

### 测试结果

所有测试结果保存在 `test-results/` 目录中：

```
test-results/
├── unit_tests.log           # 单元测试日志
├── integration_tests.log    # 集成测试日志
├── coverage_tests.log       # 覆盖率测试日志
├── coverage.out            # 覆盖率原始数据
├── coverage.html           # 覆盖率HTML报告
├── coverage_summary.txt    # 覆盖率摘要
├── lint_results.log        # 静态分析结果
├── performance/            # 性能测试结果
│   ├── memory_tests.log
│   ├── concurrent_tests.log
│   ├── response_time_tests.log
│   ├── benchmark_results.log
│   └── performance_report.md
└── security/               # 安全测试结果
    ├── input_validation_tests.log
    ├── permission_tests.log
    ├── security_config_tests.log
    ├── gosec_results.log
    ├── vulnerability_check.log
    └── security_report.md
```

## Makefile 命令

### 新增的测试命令

| 命令 | 描述 |
|------|------|
| `make test-containerized` | 运行容器化测试 |
| `make test-performance` | 运行性能测试 |
| `make test-security` | 运行安全测试 |
| `make coverage-report` | 生成覆盖率报告 |
| `make test-all` | 运行所有测试 |
| `make build-test-image` | 构建测试镜像 |
| `make clean-test-results` | 清理测试结果 |

### 使用示例

```bash
# 1. 构建测试镜像
make build-test-image

# 2. 运行基本测试
make test-containerized

# 3. 生成覆盖率报告
make coverage-report

# 4. 运行性能测试
make test-performance

# 5. 运行安全测试
make test-security

# 6. 运行所有测试
make test-all

# 7. 清理测试结果
make clean-test-results
```

## 配置文件

### 测试脚本

- `test/run_containerized_tests.sh` - 主测试脚本
- `test/performance/run_performance_tests.sh` - 性能测试脚本
- `test/security/run_security_tests.sh` - 安全测试脚本
- `test/test_core.sh` - 测试用核心维护脚本
- `test/test_rules.sh` - 测试用规则脚本

### Docker文件

- `Dockerfile.test` - 专用测试镜像
- `docker-compose.test.yml` - 测试环境配置

## CI/CD 集成

### GitHub Actions 示例

```yaml
name: Containerized Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    
    - name: Run Containerized Tests
      run: |
        cd Go
        make build-test-image
        make test-all
        
    - name: Upload Test Results
      uses: actions/upload-artifact@v2
      if: always()
      with:
        name: test-results
        path: Go/test-results/
```

## 故障排除

### 常见问题

1. **测试镜像构建失败**
   ```bash
   # 清理并重新构建
   make clean-test-results
   docker system prune -f
   make build-test-image
   ```

2. **权限问题**
   ```bash
   # 确保脚本有执行权限
   chmod +x test/*.sh
   chmod +x test/*/*.sh
   ```

3. **依赖问题**
   ```bash
   # 更新Go模块
   go mod download
   go mod tidy
   ```

4. **容器网络问题**
   ```bash
   # 重启Docker网络
   docker-compose -f docker-compose.test.yml down
   docker network prune
   ```

### 调试模式

```bash
# 启用详细输出
export DEBUG=true
make test-containerized

# 进入容器调试
docker-compose -f docker-compose.test.yml run --rm vps-tg-bot-test bash
```

## 最佳实践

1. **测试隔离**: 每个测试在独立容器中运行
2. **环境一致性**: 使用相同的Docker镜像确保测试环境一致
3. **结果持久化**: 测试结果保存在持久化卷中
4. **自动化**: 所有测试命令集成到Makefile中
5. **文档化**: 完整的测试文档和示例

## 贡献指南

添加新的测试时，请：

1. 在相应的测试脚本中添加测试逻辑
2. 更新相关文档
3. 确保测试在容器环境中正常运行
4. 添加适当的错误处理和日志记录

## 支持

如果遇到问题或需要帮助，请：

1. 检查本文档的故障排除部分
2. 查看测试日志文件
3. 在项目仓库中创建Issue