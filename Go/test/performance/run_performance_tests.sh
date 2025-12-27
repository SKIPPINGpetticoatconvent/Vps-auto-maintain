#!/bin/bash

# 性能测试脚本
set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}开始性能测试...${NC}"

# 检查环境变量
if [ "$PERFORMANCE_TEST" != "true" ]; then
    echo -e "${RED}错误: PERFORMANCE_TEST 环境变量未设置${NC}"
    exit 1
fi

# 创建性能测试结果目录
PERF_RESULTS_DIR="$COVERAGE_DIR/performance"
mkdir -p "$PERF_RESULTS_DIR"

# 设置测试参数
TEST_DURATION=${TEST_DURATION:-60s}
CONCURRENT_USERS=${CONCURRENT_USERS:-10}

echo -e "${YELLOW}运行内存使用测试...${NC}"
go test -v -timeout 2m -run TestMemoryUsage ./... 2>&1 | tee "$PERF_RESULTS_DIR/memory_tests.log"

echo -e "${YELLOW}运行并发性能测试...${NC}"
go test -v -timeout 2m -run TestConcurrent ./... 2>&1 | tee "$PERF_RESULTS_DIR/concurrent_tests.log"

echo -e "${YELLOW}运行响应时间测试...${NC}"
go test -v -timeout 2m -run TestResponseTime ./... 2>&1 | tee "$PERF_RESULTS_DIR/response_time_tests.log"

# 运行基准测试
echo -e "${YELLOW}运行基准测试...${NC}"
go test -bench=. -benchmem -timeout 5m ./... 2>&1 | tee "$PERF_RESULTS_DIR/benchmark_results.log"

# 生成性能报告
echo -e "${YELLOW}生成性能报告...${NC}"
cat > "$PERF_RESULTS_DIR/performance_report.md" << EOF
# 性能测试报告

## 测试环境
- 测试时间: $(date)
- 测试时长: $TEST_DURATION
- 并发用户数: $CONCURRENT_USERS

## 基准测试结果
EOF

# 添加基准测试结果
if [ -f "$PERF_RESULTS_DIR/benchmark_results.log" ]; then
    grep -E "(Benchmark|PASS)" "$PERF_RESULTS_DIR/benchmark_results.log" >> "$PERF_RESULTS_DIR/performance_report.md"
fi

echo -e "\n## 内存使用测试" >> "$PERF_RESULTS_DIR/performance_report.md"
if [ -f "$PERF_RESULTS_DIR/memory_tests.log" ]; then
    echo "\`\`\`" >> "$PERF_RESULTS_DIR/performance_report.md"
    grep -E "(PASS|FAIL|内存|Memory)" "$PERF_RESULTS_DIR/memory_tests.log" >> "$PERF_RESULTS_DIR/performance_report.md"
    echo "\`\`\`" >> "$PERF_RESULTS_DIR/performance_report.md"
fi

echo -e "${GREEN}性能测试完成！${NC}"
echo -e "${BLUE}性能报告保存在: $PERF_RESULTS_DIR/performance_report.md${NC}"