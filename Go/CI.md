# CI/CD 工作流说明

本项目使用 GitHub Actions 进行持续集成和持续部署。

## 工作流文件

### 1. `go-build.yml` - 完整构建工作流

**触发条件：**
- 推送到 `main` 或 `master` 分支
- 创建版本标签（`v*`）
- 手动触发（workflow_dispatch）
- Pull Request

**功能：**
- ✅ 代码检查和测试
- ✅ 多平台构建（Linux amd64/arm64, Windows amd64, macOS amd64/arm64）
- ✅ 自动创建 Release（当打 tag 时）
- ✅ Docker 镜像构建（可选）

**构建产物：**
- Linux amd64: `vps-tg-bot-linux-amd64-*.tar.gz`
- Linux arm64: `vps-tg-bot-linux-arm64-*.tar.gz`
- Windows amd64: `vps-tg-bot-windows-amd64-*.zip`
- macOS amd64: `vps-tg-bot-darwin-amd64-*.tar.gz`
- macOS arm64: `vps-tg-bot-darwin-arm64-*.tar.gz`

### 2. `go-build-simple.yml` - 简化构建工作流

**触发条件：**
- 推送到 `main` 或 `master` 分支（仅 Go 目录变更）
- Pull Request（仅 Go 目录变更）
- 手动触发

**功能：**
- ✅ 快速构建 Linux amd64 版本
- ✅ 上传构建产物

适用于快速验证和开发阶段。

### 3. `go-lint.yml` - 代码检查工作流

**触发条件：**
- 推送到 `main` 或 `master` 分支
- Pull Request

**功能：**
- ✅ 使用 golangci-lint 进行代码质量检查
- ✅ 确保代码符合规范

## 使用方法

### 本地开发

1. **使用 Makefile**（推荐）：
```bash
cd Go
make build      # 构建
make test        # 测试
make lint        # 代码检查
make fmt         # 格式化
```

2. **直接使用 Go 命令**：
```bash
cd Go
go build ./cmd/vps-tg-bot
go test ./...
go fmt ./...
```

### 创建 Release

1. **打标签**：
```bash
git tag -a v1.0.0 -m "Release version 1.0.0"
git push origin v1.0.0
```

2. **GitHub Actions 会自动：**
   - 构建所有平台的二进制文件
   - 创建 GitHub Release
   - 上传构建产物和校验和

### 手动触发工作流

1. 在 GitHub 仓库页面
2. 点击 "Actions" 标签
3. 选择对应的工作流
4. 点击 "Run workflow"

## 环境变量

### GitHub Secrets（用于 Docker 推送）

如果需要推送 Docker 镜像，需要设置：

- `DOCKER_USERNAME`: Docker Hub 用户名
- `DOCKER_PASSWORD`: Docker Hub 密码或访问令牌

设置方法：仓库 Settings → Secrets and variables → Actions → New repository secret

## 本地测试工作流

可以使用 [act](https://github.com/nektos/act) 在本地运行 GitHub Actions：

```bash
# 安装 act
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# 运行工作流
cd .github/workflows
act -l                    # 列出所有工作流
act push                  # 模拟 push 事件
act workflow_dispatch     # 手动触发
```

## 构建优化

工作流使用了以下优化：

- **缓存依赖**：使用 `actions/setup-go@v5` 的缓存功能
- **并行构建**：使用矩阵策略并行构建多个平台
- **二进制优化**：使用 `-ldflags="-s -w"` 减小二进制大小
- **压缩**：使用 tar.gz (Linux/macOS) 和 zip (Windows)

## 故障排查

### 构建失败

1. 检查 Go 版本兼容性
2. 检查依赖是否正确：`go mod verify`
3. 查看工作流日志

### Release 未创建

1. 确保标签格式正确（`v*`）
2. 检查是否有 `contents: write` 权限
3. 查看 Release 工作流的日志

### Docker 构建失败

1. 检查 Dockerfile 是否存在
2. 检查 Docker secrets 是否设置
3. 如果不需要 Docker，可以删除 docker job

## 自定义配置

### 修改构建平台

编辑 `.github/workflows/go-build.yml`，修改 `matrix.include` 部分。

### 修改 Go 版本

编辑工作流文件中的 `GO_VERSION` 环境变量。

### 添加新的检查

在 `test` job 中添加新的步骤，例如：
```yaml
- name: Run custom check
  run: |
    # 你的检查命令
```
