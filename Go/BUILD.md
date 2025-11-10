# 构建指南

## 快速构建

### 使用 Makefile（推荐）

```bash
cd Go
make build
```

### 手动构建

```bash
cd Go
go build -o vps-tg-bot ./cmd/vps-tg-bot
```

## 多平台构建

### 使用 Makefile

```bash
make build-all
```

这会构建以下平台：
- Linux amd64
- Linux arm64
- Windows amd64
- macOS amd64
- macOS arm64

### 手动构建

```bash
# Linux amd64
GOOS=linux GOARCH=amd64 go build -o vps-tg-bot-linux-amd64 ./cmd/vps-tg-bot

# Linux arm64
GOOS=linux GOARCH=arm64 go build -o vps-tg-bot-linux-arm64 ./cmd/vps-tg-bot

# Windows amd64
GOOS=windows GOARCH=amd64 go build -o vps-tg-bot-windows-amd64.exe ./cmd/vps-tg-bot

# macOS amd64
GOOS=darwin GOARCH=amd64 go build -o vps-tg-bot-darwin-amd64 ./cmd/vps-tg-bot

# macOS arm64
GOOS=darwin GOARCH=arm64 go build -o vps-tg-bot-darwin-arm64 ./cmd/vps-tg-bot
```

## 优化构建

### 减小二进制大小

```bash
go build -ldflags="-s -w" -trimpath -o vps-tg-bot ./cmd/vps-tg-bot
```

参数说明：
- `-ldflags="-s -w"`: 去除符号表和调试信息
- `-trimpath`: 去除文件系统路径

### 添加版本信息

在 `main.go` 中添加版本变量，然后构建时注入：

```bash
go build -ldflags="-X main.Version=$(git describe --tags) -X main.BuildTime=$(date -u '+%Y-%m-%d_%H:%M:%S')" -o vps-tg-bot ./cmd/vps-tg-bot
```

## Docker 构建

### 构建镜像

```bash
cd Go
docker build -t vps-tg-bot:latest .
```

### 运行容器

```bash
docker run -d \
  --name vps-tg-bot \
  -e TG_TOKEN="your_token" \
  -e TG_CHAT_ID="your_chat_id" \
  --restart unless-stopped \
  vps-tg-bot:latest
```

### 使用 docker-compose

```bash
cd Go
# 创建 .env 文件
echo "TG_TOKEN=your_token" > .env
echo "TG_CHAT_ID=your_chat_id" >> .env

# 启动
docker-compose up -d
```

## 交叉编译注意事项

1. **CGO**: 如果使用了 CGO，需要安装对应的交叉编译工具链
2. **静态链接**: 使用 `CGO_ENABLED=0` 可以静态链接，避免依赖问题
3. **测试**: 交叉编译的二进制文件需要在目标平台上测试

## 构建检查清单

- [ ] 代码格式化：`make fmt`
- [ ] 代码检查：`make lint` 或 `make vet`
- [ ] 运行测试：`make test`
- [ ] 构建二进制：`make build`
- [ ] 测试运行：设置环境变量后运行
- [ ] 多平台构建：`make build-all`
- [ ] 检查二进制大小和功能

## 常见问题

### 构建失败：找不到模块

```bash
go mod download
go mod tidy
```

### 构建失败：版本不兼容

检查 `go.mod` 中的 Go 版本要求，确保本地 Go 版本符合要求。

### 二进制文件太大

使用优化参数：
```bash
go build -ldflags="-s -w" -trimpath -o vps-tg-bot ./cmd/vps-tg-bot
```

### 交叉编译失败

确保设置了正确的环境变量：
```bash
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build ...
```
