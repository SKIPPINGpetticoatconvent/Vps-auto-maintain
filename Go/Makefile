# Makefile for VPS Telegram Bot

.PHONY: help build clean test lint fmt vet run install

# 变量定义
BINARY_NAME=vps-tg-bot
MAIN_PATH=./cmd/vps-tg-bot
VERSION?=$(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
BUILD_TIME=$(shell date -u '+%Y-%m-%d_%H:%M:%S')
LDFLAGS=-ldflags "-s -w -X main.Version=$(VERSION) -X main.BuildTime=$(BUILD_TIME)"

# 默认目标
help:
	@echo "可用的命令:"
	@echo "  make build      - 构建二进制文件"
	@echo "  make clean      - 清理构建文件"
	@echo "  make test       - 运行测试"
	@echo "  make lint       - 运行代码检查"
	@echo "  make fmt        - 格式化代码"
	@echo "  make vet        - 运行 go vet"
	@echo "  make run        - 运行程序（需要设置环境变量）"
	@echo "  make install    - 安装到系统"
	@echo "  make build-all  - 构建所有平台"

# 构建
build:
	@echo "构建 $(BINARY_NAME)..."
	@go build $(LDFLAGS) -o $(BINARY_NAME) $(MAIN_PATH)
	@echo "构建完成: $(BINARY_NAME)"

# 构建所有平台
build-all:
	@echo "构建所有平台..."
	@GOOS=linux GOARCH=amd64 go build $(LDFLAGS) -o $(BINARY_NAME)-linux-amd64 $(MAIN_PATH)
	@GOOS=linux GOARCH=arm64 go build $(LDFLAGS) -o $(BINARY_NAME)-linux-arm64 $(MAIN_PATH)
	@GOOS=windows GOARCH=amd64 go build $(LDFLAGS) -o $(BINARY_NAME)-windows-amd64.exe $(MAIN_PATH)
	@GOOS=darwin GOARCH=amd64 go build $(LDFLAGS) -o $(BINARY_NAME)-darwin-amd64 $(MAIN_PATH)
	@GOOS=darwin GOARCH=arm64 go build $(LDFLAGS) -o $(BINARY_NAME)-darwin-arm64 $(MAIN_PATH)
	@echo "所有平台构建完成"

# 清理
clean:
	@echo "清理构建文件..."
	@rm -f $(BINARY_NAME)
	@rm -f $(BINARY_NAME)-*
	@rm -f *.tar.gz *.zip
	@go clean
	@echo "清理完成"

# 测试
test:
	@echo "运行测试..."
	@go test -v ./...

# 代码检查
lint:
	@echo "运行代码检查..."
	@if command -v golangci-lint > /dev/null; then \
		golangci-lint run; \
	else \
		echo "golangci-lint 未安装，跳过"; \
	fi

# 格式化
fmt:
	@echo "格式化代码..."
	@go fmt ./...
	@if command -v goimports > /dev/null; then \
		goimports -w .; \
	fi

# go vet
vet:
	@echo "运行 go vet..."
	@go vet ./...

# 运行
run:
	@echo "运行程序..."
	@go run $(MAIN_PATH)

# 安装
install: build
	@echo "安装到系统..."
	@sudo cp $(BINARY_NAME) /usr/local/bin/
	@echo "安装完成"

# 开发模式（需要环境变量）
dev:
	@echo "开发模式运行..."
	@go run $(MAIN_PATH)

# 检查依赖
deps:
	@echo "检查依赖..."
	@go mod download
	@go mod verify
	@go mod tidy

# 更新依赖
deps-update:
	@echo "更新依赖..."
	@go get -u ./...
	@go mod tidy
