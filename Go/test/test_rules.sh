#!/bin/bash

# 测试用维护规则脚本
# 这个脚本用于容器化测试环境

set -e

# 日志函数
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] [TEST] $1"
}

log "开始执行维护规则检查（测试模式）"

# 模拟规则检查
log "检查系统安全规则..."

# 模拟防火墙规则检查
log "检查防火墙规则..."
echo "Test: 防火墙规则检查 - 端口22, 80, 443 开放"

# 模拟用户账户检查
log "检查用户账户安全..."
echo "Test: 用户账户检查 - 发现 1 个用户账户 (testuser)"

# 模拟服务状态检查
log "检查关键服务状态..."
echo "Test: 关键服务检查 - 所有服务正常运行"

# 模拟磁盘空间检查
log "检查磁盘空间规则..."
DISK_USAGE=$(df -h / | tail -1 | awk '{print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -lt 80 ]; then
    log "磁盘使用率正常: ${DISK_USAGE}%"
else
    log "警告: 磁盘使用率过高: ${DISK_USAGE}%"
fi

# 模拟内存检查
log "检查内存使用规则..."
MEMORY_USAGE=$(free | grep Mem | awk '{printf("%.1f", $3/$2 * 100.0)}')
log "内存使用率: ${MEMORY_USAGE}%"

# 模拟系统更新检查
log "检查系统更新规则..."
echo "Test: 系统更新检查 - 模拟 0 个可用更新"

log "维护规则检查完成（测试模式）"
exit 0