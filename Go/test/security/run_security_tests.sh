#!/bin/bash

# 安全测试脚本
set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}开始安全测试...${NC}"

# 检查环境变量
if [ "$SECURITY_TEST" != "true" ]; then
    echo -e "${RED}错误: SECURITY_TEST 环境变量未设置${NC}"
    exit 1
fi

# 创建安全测试结果目录
SECURITY_RESULTS_DIR="$COVERAGE_DIR/security"
mkdir -p "$SECURITY_RESULTS_DIR"

echo -e "${YELLOW}运行输入验证测试...${NC}"
go test -v -timeout 1m -run TestInputValidation ./... 2>&1 | tee "$SECURITY_RESULTS_DIR/input_validation_tests.log"

echo -e "${YELLOW}运行权限控制测试...${NC}"
go test -v -timeout 1m -run TestPermission ./... 2>&1 | tee "$SECURITY_RESULTS_DIR/permission_tests.log"

echo -e "${YELLOW}运行安全配置测试...${NC}"
go test -v -timeout 1m -run TestSecurityConfig ./... 2>&1 | tee "$SECURITY_RESULTS_DIR/security_config_tests.log"

# 静态安全分析
echo -e "${YELLOW}运行静态安全分析...${NC}"
if command -v gosec >/dev/null 2>&1; then
    gosec ./... 2>&1 | tee "$SECURITY_RESULTS_DIR/gosec_results.log" || true
else
    echo "gosec 未安装，跳过静态安全分析"
fi

# 检查依赖安全性
echo -e "${YELLOW}检查依赖安全性...${NC}"
if command -v govulncheck >/dev/null 2>&1; then
    govulncheck ./... 2>&1 | tee "$SECURITY_RESULTS_DIR/vulnerability_check.log" || true
else
    echo "govulncheck 未安装，跳过漏洞检查"
fi

# 生成安全报告
echo -e "${YELLOW}生成安全报告...${NC}"
cat > "$SECURITY_RESULTS_DIR/security_report.md" << EOF
# 安全测试报告

## 测试时间
$(date)

## 测试项目
- 输入验证测试
- 权限控制测试
- 安全配置测试
- 静态安全分析
- 依赖漏洞检查

## 测试结果
EOF

# 添加测试结果摘要
for test_log in "$SECURITY_RESULTS_DIR"/*_tests.log; do
    if [ -f "$test_log" ]; then
        echo -e "\n### $(basename "$test_log" .log)" >> "$SECURITY_RESULTS_DIR/security_report.md"
        echo "\`\`\`" >> "$SECURITY_RESULTS_DIR/security_report.md"
        grep -E "(PASS|FAIL)" "$test_log" >> "$SECURITY_RESULTS_DIR/security_report.md"
        echo "\`\`\`" >> "$SECURITY_RESULTS_DIR/security_report.md"
    fi
done

# 添加静态分析结果
if [ -f "$SECURITY_RESULTS_DIR/gosec_results.log" ]; then
    echo -e "\n### 静态安全分析 (gosec)" >> "$SECURITY_RESULTS_DIR/security_report.md"
    echo "\`\`\`" >> "$SECURITY_RESULTS_DIR/security_report.md"
    grep -E "(HIGH|MEDIUM|LOW)" "$SECURITY_RESULTS_DIR/gosec_results.log" >> "$SECURITY_RESULTS_DIR/security_report.md"
    echo "\`\`\`" >> "$SECURITY_RESULTS_DIR/security_report.md"
fi

echo -e "${GREEN}安全测试完成！${NC}"
echo -e "${BLUE}安全报告保存在: $SECURITY_RESULTS_DIR/security_report.md${NC}"