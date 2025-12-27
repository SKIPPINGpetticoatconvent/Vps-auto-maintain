#!/bin/bash

# 容器化测试脚本
set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}开始容器化测试...${NC}"

# 检查环境变量
if [ "$TEST_MODE" != "true" ]; then
    echo -e "${RED}错误: TEST_MODE 环境变量未设置${NC}"
    exit 1
fi

# 创建测试结果目录
mkdir -p "$COVERAGE_DIR"

# 设置测试超时
TEST_TIMEOUT=${TEST_TIMEOUT:-30s}

echo -e "${YELLOW}运行单元测试...${NC}"
go test -timeout "$TEST_TIMEOUT" -v ./... 2>&1 | tee "$COVERAGE_DIR/unit_tests.log"

echo -e "${YELLOW}运行集成测试...${NC}"
go test -timeout "$TEST_TIMEOUT" -v -tags integration ./... 2>&1 | tee "$COVERAGE_DIR/integration_tests.log"

echo -e "${YELLOW}运行覆盖率测试...${NC}"
go test -timeout "$TEST_TIMEOUT" -v -coverprofile="$COVERAGE_DIR/coverage.out" ./... 2>&1 | tee "$COVERAGE_DIR/coverage_tests.log"

# 生成覆盖率报告
if [ -f "$COVERAGE_DIR/coverage.out" ]; then
    echo -e "${YELLOW}生成覆盖率报告...${NC}"
    go tool cover -html="$COVERAGE_DIR/coverage.out" -o "$COVERAGE_DIR/coverage.html" 2>/dev/null || true
    go tool cover -func="$COVERAGE_DIR/coverage.out" > "$COVERAGE_DIR/coverage_summary.txt" 2>/dev/null || true
fi

# 运行静态代码分析（如果有 golangci-lint）
if command -v golangci-lint >/dev/null 2>&1; then
    echo -e "${YELLOW}运行静态代码分析...${NC}"
    golangci-lint run --timeout=5m 2>&1 | tee "$COVERAGE_DIR/lint_results.log" || true
fi

echo -e "${GREEN}容器化测试完成！${NC}"
echo -e "${BLUE}测试结果保存在: $COVERAGE_DIR${NC}"

# 输出测试摘要
echo -e "\n${BLUE}=== 测试摘要 ===${NC}"
if [ -f "$COVERAGE_DIR/coverage_summary.txt" ]; then
    echo -e "${GREEN}覆盖率摘要:${NC}"
    tail -n 10 "$COVERAGE_DIR/coverage_summary.txt"
fi

# 检查测试是否通过
if grep -q "FAIL" "$COVERAGE_DIR/unit_tests.log" 2>/dev/null || \
   grep -q "FAIL" "$COVERAGE_DIR/integration_tests.log" 2>/dev/null; then
    echo -e "${RED}某些测试失败！${NC}"
    exit 1
else
    echo -e "${GREEN}所有测试通过！${NC}"
fi