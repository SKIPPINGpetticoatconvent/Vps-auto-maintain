#!/bin/bash
set -e

# 定义颜色
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=================================================="
echo "   VPS Telegram Bot 交互式配置功能测试"
echo "=================================================="

# 编译项目
echo -e "${GREEN}正在编译项目...${NC}"
cd Rust/vps-tg-bot
cargo build
BINARY_PATH="./target/debug/vps-tg-bot"
cd ../..

if [ ! -f "Rust/vps-tg-bot/$BINARY_PATH" ]; then
    echo -e "${RED}❌ 编译失败，未找到二进制文件${NC}"
    exit 1
fi

echo -e "${GREEN}✅ 编译成功${NC}"

# 测试场景 1: 使用 expect 模拟交互式输入
echo -e "\n${GREEN}测试场景 1: 模拟交互式输入${NC}"

# 创建 expect 脚本
cat > test_interactive.exp <<EOF
#!/usr/bin/expect -f

set timeout 10
set binary_path "Rust/vps-tg-bot/target/debug/vps-tg-bot"

spawn \$binary_path run

expect {
    "检测到首次运行或配置丢失" {
        send_user "\n✅ 检测到配置丢失提示\n"
    }
    timeout {
        send_user "\n❌ 未检测到配置丢失提示 (超时)\n"
        exit 1
    }
}

expect {
    "请输入 BOT_TOKEN:" {
        send "123456:ABC-DEF1234ghIkl-zyx57W2v1u12345\r"
    }
    timeout {
        send_user "\n❌ 未提示输入 Token\n"
        exit 1
    }
}

expect {
    "请输入 CHAT_ID:" {
        send "123456789\r"
    }
    timeout {
        send_user "\n❌ 未提示输入 Chat ID\n"
        exit 1
    }
}

# 此时应该会生成 config.enc，然后尝试加载配置并连接 Telegram
# 由于 Token 是假的，连接必然失败，但我们的目的是验证配置流程
expect {
    "配置初始化完成" {
        send_user "\n✅ 配置初始化流程完成\n"
    }
    "配置加载成功" {
        send_user "\n✅ 配置加载成功\n"
    }
    timeout {
        # 即使连接失败，我们只要确认配置已生成即可
        send_user "\n⚠️  配置流程可能已完成，但连接失败（预期行为）\n"
    }
}

# 检查配置文件是否生成
if {[file exists "config.enc"]} {
    send_user "\n✅ 配置文件 config.enc 已生成\n"
} else {
    send_user "\n❌ 配置文件 config.enc 未生成\n"
    exit 1
}
EOF

chmod +x test_interactive.exp

# 清理旧的配置文件（如果存在）
rm -f config.enc
rm -f Rust/vps-tg-bot/config.enc

# 运行 expect 脚本
# 注意：这需要系统安装 expect。如果未安装，我们只能进行基本的检查
if command -v expect &> /dev/null; then
    ./test_interactive.exp
else
    echo -e "${RED}⚠️  未找到 expect 命令，跳过交互式模拟测试${NC}"
    echo "请手动运行 cargo run -- run 进行测试"
fi

# 清理
rm -f test_interactive.exp
rm -f config.enc

echo -e "\n${GREEN}测试完成！${NC}"
