# 多阶段构建 Dockerfile
# 阶段1: 构建
FROM golang:1.21-alpine AS builder

WORKDIR /build

# 安装必要的工具
RUN apk add --no-cache git make

# 复制 go mod 文件
COPY go.mod go.sum ./
RUN go mod download

# 复制源代码
COPY . .

# 构建二进制文件
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -trimpath -o vps-tg-bot ./cmd/vps-tg-bot

# 阶段2: 运行
FROM alpine:latest

# 安装必要的运行时依赖
RUN apk --no-cache add ca-certificates tzdata bash

# 创建非 root 用户
RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /build/vps-tg-bot /app/vps-tg-bot

# 复制维护脚本（如果存在）
COPY --from=builder /build/vps-tg-bot-install.sh /app/vps-tg-bot-install.sh 2>/dev/null || true

# 设置权限
RUN chmod +x /app/vps-tg-bot

# 切换到非 root 用户
USER appuser

# 暴露端口（如果需要）
# EXPOSE 8080

# 设置环境变量
ENV TG_TOKEN=""
ENV TG_CHAT_ID=""
ENV CORE_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
ENV RULES_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"

# 运行程序
ENTRYPOINT ["/app/vps-tg-bot"]
