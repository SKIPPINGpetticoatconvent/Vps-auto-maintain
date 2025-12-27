#!/bin/bash
# -------------------------------------------------------------------
# 下载预编译二进制修复脚本
# 解决 "无法获取下载链接" 问题
# -------------------------------------------------------------------

set -e

# 彩色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_message() {
  echo ""
  echo "============================================================"
  echo "$1"
  echo "============================================================"
}
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error()   { echo -e "${RED}❌ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }

print_message "下载预编译二进制修复工具"

# 检查网络连接
check_network() {
  print_message "检查网络连接"
  
  local test_urls=(
    "https://api.github.com"
  )
  
  local working_urls=()
  
  for url in "${test_urls[@]}"; do
    print_warning "测试连接: $url"
    if curl -s --max-time 5 "$url" >/dev/null 2>&1; then
      print_success "✅ $url 连接正常"
      working_urls+=("$url")
    else
      print_error "❌ $url 连接失败"
    fi
  done
  
  if [ ${#working_urls[@]} -eq 0 ]; then
    print_error "所有网络测试都失败，建议检查网络连接"
    return 1
  else
    print_success "找到 ${#working_urls[@]} 个可用的网络源"
    return 0
  fi
}

# 测试 GitHub 仓库访问
test_repos() {
  print_message "测试 GitHub 仓库访问"
  
  local repos=(
    "FTDRTD/Vps-auto-maintain"
    "SKIPPINGpetticoatconvent/Vps-auto-maintain"
  )
  
  local working_repos=()
  
  for repo in "${repos[@]}"; do
    print_warning "测试仓库: $repo"
    
    if curl -s --max-time 10 "https://api.github.com/repos/$repo/releases/latest" >/dev/null 2>&1; then
      print_success "✅ $repo 可访问"
      working_repos+=("$repo")
    else
      print_error "❌ $repo 不可访问"
    fi
  done
  
  if [ ${#working_repos[@]} -eq 0 ]; then
    print_error "所有仓库都不可访问"
    return 1
  else
    print_success "找到 ${#working_repos[@]} 个可用的仓库"
    return 0
  fi
}

# 下载测试
download_test() {
  print_message "下载测试"
  
  local repos=("FTDRTD/Vps-auto-maintain" "SKIPPINGpetticoatconvent/Vps-auto-maintain")
  
  for repo in "${repos[@]}"; do
    local api_url="api.github.com/repos/${repo}/releases/latest"
    print_warning "测试 API: $api_url"
    
    if response=$(curl -s --max-time 10 "$api_url" 2>/dev/null); then
      if echo "$response" | grep -q "tag_name"; then
        download_url=$(echo "$response" | grep -oE '"browser_download_url":\s*"([^"]+vps-tg-bot-linux-amd64[^"]*)' | cut -d'"' -f4 | head -n1)
        if [ -n "$download_url" ]; then
          print_success "✅ 找到下载链接: $download_url"
          
          # 测试下载
          print_warning "测试下载..."
          if curl -L --max-time 30 -o /tmp/test_binary "$download_url" 2>/dev/null; then
            if [ -s /tmp/test_binary ]; then
              print_success "✅ 下载测试成功"
              rm -f /tmp/test_binary
              echo ""
              echo "============================================================"
              print_success "修复方案可用！"
              echo "============================================================"
              print_warning "运行以下命令重新部署："
              echo "cd $(pwd)"
              echo "bash deploy.sh"
              return 0
            else
              print_error "❌ 下载的文件为空"
              rm -f /tmp/test_binary
            fi
          else
            print_error "❌ 下载测试失败"
            rm -f /tmp/test_binary
          fi
        else
          print_error "❌ 未找到有效的下载链接"
        fi
      else
        print_error "❌ API 响应格式错误"
      fi
    else
      print_error "❌ 无法访问 API"
    fi
  done
  
  print_error "所有下载测试都失败"
  return 1
}

# 本地编译方案
local_compile() {
  print_message "本地编译方案"
  print_warning "由于无法下载预编译二进制，提供本地编译方案："
  echo ""
  echo "1. 安装 Go 环境："
  echo "   curl -L https://go.dev/dl/go1.21.5.linux-amd64.tar.gz | tar -xzC /usr/local"
  echo "   export PATH=\$PATH:/usr/local/go/bin"
  echo ""
  echo "2. 编译项目："
  echo "   cd $(pwd)"
  echo "   go mod tidy"
  echo "   go build -o vps-tg-bot ./cmd/vps-tg-bot"
  echo ""
  echo "3. 使用编译的二进制文件替换 deploy.sh 中的下载步骤"
  
  # 检查当前目录是否有 Go 源码
  if [ -f "$(pwd)/go.mod" ] && [ -d "$(pwd)/cmd" ]; then
    print_success "检测到 Go 源码，询问是否立即编译"
    read -p "是否立即尝试本地编译？(y/N): " compile_now
    if [[ "$compile_now" =~ ^[Yy]$ ]]; then
      compile_project
    fi
  fi
}

# 编译项目
compile_project() {
  print_message "开始本地编译"
  
  # 安装 Go（如果需要）
  if ! command -v go &>/dev/null; then
    print_warning "Go 未安装，开始安装..."
    GO_VERSION="1.21.5"
    GO_URL="https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz"
    
    if curl -L -o /tmp/go.tar.gz "$GO_URL"; then
      print_warning "解压 Go 到 /usr/local..."
      tar -C /usr/local -xzf /tmp/go.tar.gz
      rm -f /tmp/go.tar.gz
      
      export PATH=$PATH:/usr/local/go/bin
      print_success "Go 安装完成"
    else
      print_error "Go 下载失败"
      return 1
    fi
  fi
  
  # 编译项目
  print_warning "开始编译项目..."
  if go mod tidy && go build -o vps-tg-bot ./cmd/vps-tg-bot; then
    print_success "编译成功！"
    print_success "生成的二进制文件: $(pwd)/vps-tg-bot"
    print_warning "现在可以运行: bash deploy.sh"
    print_warning "脚本将自动检测并使用本地二进制文件"
  else
    print_error "编译失败，请检查错误信息"
    return 1
  fi
}

# 主要逻辑
main() {
  echo "这个工具将帮助您解决下载预编译二进制的问题"
  echo ""
  
  if ! check_network; then
    print_error "网络连接问题，请检查网络设置"
    local_compile
    return 1
  fi
  
  if ! test_repos; then
    print_error "仓库访问问题"
    local_compile
    return 1
  fi
  
  if download_test; then
    return 0
  else
    print_error "下载测试失败"
    local_compile
    return 1
  fi
}

# 运行主函数
main "$@"