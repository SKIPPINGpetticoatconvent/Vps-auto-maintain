#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Telegram端口监控机器人 - 终极安全版
基于detect_ports_ultimate.sh的功能，创建交互式Telegram机器人

功能：
- 实时检测Xray和Sing-box端口状态
- 自动检测和配置防火墙（UFW/Firewalld）
- 端口安全锁定功能
- 查询防火墙配置
- 监控SSH端口
- 通过Telegram交互界面操作
- 自动清理未知端口
"""

import os
import sys
import json
import subprocess
import re
import time
from datetime import datetime
from telegram import Update
from telegram.ext import Application, CommandHandler, ContextTypes, MessageHandler, filters
import logging
import asyncio
from typing import List, Dict, Optional, Tuple

# 配置日志
logging.basicConfig(
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    level=logging.INFO
)
logger = logging.getLogger(__name__)

class FirewallManager:
    """防火墙管理器"""

    def __init__(self):
        self.firewall_type = self.detect_firewall()

    def detect_firewall(self) -> str:
        """检测防火墙类型"""
        try:
            # 检查firewalld
            result = subprocess.run(['systemctl', 'is-active', 'firewalld'],
                                  capture_output=True, text=True, timeout=10)
            if result.returncode == 0 and 'active' in result.stdout:
                return 'firewalld'
        except:
            pass

        try:
            # 检查ufw
            result = subprocess.run(['ufw', 'status'],
                                  capture_output=True, text=True, timeout=10)
            if 'Status: active' in result.stdout:
                return 'ufw'
        except:
            pass

        return 'none'

    def install_firewall(self) -> str:
        """自动安装防火墙"""
        print("🔧 未检测到活跃防火墙，开始自动安装...")

        try:
            # 检测操作系统
            if os.path.exists('/etc/os-release'):
                with open('/etc/os-release', 'r') as f:
                    os_info = f.read()

                if 'ubuntu' in os_info.lower() or 'debian' in os_info.lower():
                    print("📦 检测到Debian/Ubuntu系统，安装UFW...")
                    subprocess.run(['sudo', 'apt-get', 'update'], check=True,
                                 capture_output=True, timeout=60)
                    subprocess.run(['sudo', 'apt-get', 'install', '-y', 'ufw'], check=True,
                                 capture_output=True, timeout=60)

                    # 配置UFW
                    subprocess.run(['sudo', 'ufw', '--force', 'reset'], check=True,
                                 capture_output=True, timeout=30)
                    subprocess.run(['sudo', 'ufw', 'default', 'deny', 'incoming'], check=True,
                                 capture_output=True, timeout=10)
                    subprocess.run(['sudo', 'ufw', 'default', 'allow', 'outgoing'], check=True,
                                 capture_output=True, timeout=10)
                    subprocess.run(['sudo', 'ufw', '--force', 'enable'], check=True,
                                 capture_output=True, timeout=10)

                    print("✅ UFW安装并配置成功")
                    return 'ufw'

                elif 'centos' in os_info.lower() or 'rhel' in os_info.lower() or 'fedora' in os_info.lower():
                    print("📦 检测到RHEL/CentOS系统，安装firewalld...")

                    if 'dnf' in os_info:
                        subprocess.run(['sudo', 'dnf', 'install', '-y', 'firewalld'], check=True,
                                     capture_output=True, timeout=60)
                    else:
                        subprocess.run(['sudo', 'yum', 'install', '-y', 'firewalld'], check=True,
                                     capture_output=True, timeout=60)

                    subprocess.run(['sudo', 'systemctl', 'enable', '--now', 'firewalld'], check=True,
                                 capture_output=True, timeout=30)

                    print("✅ firewalld安装并启用成功")
                    return 'firewalld'

            print("❌ 不支持的操作系统")
            return 'none'

        except subprocess.CalledProcessError as e:
            print(f"❌ 防火墙安装失败: {e}")
            return 'none'
        except subprocess.TimeoutExpired:
            print("❌ 防火墙安装超时")
            return 'none'

    def add_rule(self, port: int, protocol: str = 'tcp') -> bool:
        """添加防火墙规则"""
        if self.firewall_type == 'none':
            return False

        try:
            if self.firewall_type == 'firewalld':
                cmd = ['sudo', 'firewall-cmd', '--permanent', '--add-port', f'{port}/{protocol}']
                subprocess.run(cmd, check=True, capture_output=True, timeout=10)
                subprocess.run(['sudo', 'firewall-cmd', '--reload'], check=True,
                             capture_output=True, timeout=10)
            elif self.firewall_type == 'ufw':
                cmd = ['sudo', 'ufw', 'allow', str(port)]
                subprocess.run(cmd, check=True, capture_output=True, timeout=10)

            return True
        except subprocess.CalledProcessError as e:
            logger.error(f"添加防火墙规则失败 {port}/{protocol}: {e}")
            return False
        except subprocess.TimeoutExpired:
            logger.error(f"添加防火墙规则超时 {port}/{protocol}")
            return False

    def remove_rule(self, port: int, protocol: str = 'tcp') -> bool:
        """移除防火墙规则"""
        if self.firewall_type == 'none':
            return False

        try:
            if self.firewall_type == 'firewalld':
                cmd = ['sudo', 'firewall-cmd', '--permanent', '--remove-port', f'{port}/{protocol}']
                subprocess.run(cmd, check=True, capture_output=True, timeout=10)
                subprocess.run(['sudo', 'firewall-cmd', '--reload'], check=True,
                             capture_output=True, timeout=10)
            elif self.firewall_type == 'ufw':
                cmd = ['sudo', 'ufw', 'delete', 'allow', str(port)]
                subprocess.run(cmd, check=True, capture_output=True, timeout=10)

            return True
        except subprocess.CalledProcessError as e:
            logger.error(f"移除防火墙规则失败 {port}/{protocol}: {e}")
            return False
        except subprocess.TimeoutExpired:
            logger.error(f"移除防火墙规则超时 {port}/{protocol}")
            return False

    def get_current_rules(self) -> Dict[str, List[int]]:
        """获取当前防火墙规则"""
        rules = {'tcp': [], 'udp': []}

        try:
            if self.firewall_type == 'firewalld':
                result = subprocess.run(['sudo', 'firewall-cmd', '--list-ports'],
                                      capture_output=True, text=True, timeout=10)
                if result.returncode == 0:
                    ports = result.stdout.strip().split()
                    for port_info in ports:
                        if '/' in port_info:
                            port, protocol = port_info.split('/')
                            if protocol in rules:
                                rules[protocol].append(int(port))

            elif self.firewall_type == 'ufw':
                result = subprocess.run(['sudo', 'ufw', 'status', 'numbered'],
                                      capture_output=True, text=True, timeout=10)
                if result.returncode == 0:
                    for line in result.stdout.split('\n'):
                        if 'ALLOW' in line and line.strip().endswith('/tcp'):
                            match = re.search(r'(\d+)/tcp', line)
                            if match:
                                rules['tcp'].append(int(match.group(1)))
                        elif 'ALLOW' in line and line.strip().endswith('/udp'):
                            match = re.search(r'(\d+)/udp', line)
                            if match:
                                rules['udp'].append(int(match.group(1)))

        except Exception as e:
            logger.error(f"获取防火墙规则失败: {e}")

        return rules

    def reset_to_secure(self, allowed_ports: List[int]) -> bool:
        """重置防火墙为安全状态，只保留指定端口"""
        print("🔒 开始安全锁定防火墙...")

        try:
            if self.firewall_type == 'firewalld':
                # 移除所有非必需规则
                current_rules = self.get_current_rules()
                firewall_changed = False

                for protocol in ['tcp', 'udp']:
                    for port in current_rules[protocol]:
                        if port not in allowed_ports:
                            print(f"➖ 移除端口 {port}/{protocol}")
                            self.remove_rule(port, protocol)
                            firewall_changed = True

                # 添加必需端口
                for port in allowed_ports:
                    if port not in current_rules['tcp']:
                        print(f"➕ 添加端口 {port}/tcp")
                        self.add_rule(port, 'tcp')
                    if port not in current_rules['udp']:
                        print(f"➕ 添加端口 {port}/udp")
                        self.add_rule(port, 'udp')

                if firewall_changed:
                    subprocess.run(['sudo', 'firewall-cmd', '--reload'], check=True,
                                 capture_output=True, timeout=10)

            elif self.firewall_type == 'ufw':
                print("⚠️ UFW将被重置，仅保留必需端口！")
                print("   操作将在3秒后继续...")
                time.sleep(3)

                # 重置UFW
                subprocess.run(['sudo', 'ufw', '--force', 'reset'], check=True,
                             capture_output=True, timeout=30)
                subprocess.run(['sudo', 'ufw', 'default', 'deny', 'incoming'], check=True,
                             capture_output=True, timeout=10)
                subprocess.run(['sudo', 'ufw', 'default', 'allow', 'outgoing'], check=True,
                             capture_output=True, timeout=10)

                # 添加必需端口
                for port in allowed_ports:
                    print(f"➕ 允许端口: {port}")
                    subprocess.run(['sudo', 'ufw', 'allow', str(port)], check=True,
                                 capture_output=True, timeout=10)

                subprocess.run(['sudo', 'ufw', '--force', 'enable'], check=True,
                             capture_output=True, timeout=10)

            print("✅ 防火墙安全锁定完成")
            return True

        except Exception as e:
            logger.error(f"防火墙安全锁定失败: {e}")
            return False

class PortMonitorBot:
    """端口监控机器人"""

    def __init__(self, token: str, allowed_chat_ids: list = None):
        self.token = token
        self.allowed_chat_ids = allowed_chat_ids or []
        self.application = None
        self.firewall_manager = FirewallManager()
        self.notification_enabled = True

    def get_timezone(self) -> str:
        """获取系统时区"""
        try:
            result = subprocess.run(['timedatectl'], capture_output=True, text=True, timeout=5)
            for line in result.stdout.split('\n'):
                if 'Time zone:' in line:
                    return line.split(':')[1].strip()
        except:
            pass

        try:
            with open('/etc/timezone', 'r') as f:
                return f.read().strip()
        except:
            return 'Etc/UTC'

    def get_process_ports(self, process_name: str) -> list:
        """获取进程使用的端口"""
        ports = []
        try:
            # 检查进程是否运行
            result = subprocess.run(['pgrep', '-f', process_name],
                                  capture_output=True, text=True, timeout=5)
            if result.returncode != 0:
                return ports

            # 使用ss命令检测端口
            try:
                result = subprocess.run(['ss', '-tlnp'],
                                      capture_output=True, text=True, timeout=10)
                for line in result.stdout.split('\n'):
                    if process_name in line:
                        match = re.search(r':(\d+)\s', line)
                        if match:
                            ports.append(int(match.group(1)))
            except FileNotFoundError:
                # 如果没有ss，使用netstat
                result = subprocess.run(['netstat', '-tlnp'],
                                      capture_output=True, text=True, timeout=10)
                for line in result.stdout.split('\n'):
                    if process_name in line:
                        match = re.search(r':(\d+)\s', line)
                        if match:
                            ports.append(int(match.group(1)))
        except Exception as e:
            logger.error(f"获取进程端口失败: {e}")

        return list(set(ports))  # 去重

    def parse_config_ports(self, config_file: str) -> list:
        """从配置文件解析端口"""
        ports = []
        if not os.path.exists(config_file):
            return ports

        try:
            with open(config_file, 'r', encoding='utf-8') as f:
                config = json.load(f)

            def extract_ports(obj):
                if isinstance(obj, dict):
                    for key, value in obj.items():
                        if key in ['listen_port', 'port'] and isinstance(value, int):
                            ports.append(value)
                        elif isinstance(value, (dict, list)):
                            extract_ports(value)
                elif isinstance(obj, list):
                    for item in obj:
                        extract_ports(item)

            extract_ports(config)
        except Exception as e:
            logger.error(f"解析配置文件失败 {config_file}: {e}")

        return list(set(ports))  # 去重

    def get_ssh_port(self) -> int:
        """获取SSH端口"""
        try:
            with open('/etc/ssh/sshd_config', 'r') as f:
                for line in f:
                    if line.strip().lower().startswith('port '):
                        port = int(line.split()[1])
                        return port
        except:
            pass
        return 22

    def get_all_service_ports(self) -> List[int]:
        """获取所有服务端口"""
        all_ports = []
        ssh_port = self.get_ssh_port()
        all_ports.append(ssh_port)

        # Xray端口
        try:
            result = subprocess.run(['pgrep', '-f', 'xray'],
                                  capture_output=True, text=True, timeout=5)
            if result.returncode == 0:
                xray_ports = self.get_process_ports('xray')
                all_ports.extend(xray_ports)
                print(f"✅ 检测到Xray运行端口: {xray_ports}")
        except Exception as e:
            print(f"❌ 检测Xray失败: {e}")

        # Sing-box端口
        try:
            result = subprocess.run(['pgrep', '-f', 'sing-box'],
                                  capture_output=True, text=True, timeout=5)
            if result.returncode == 0:
                sb_ports = self.get_process_ports('sing-box')
                if not sb_ports:
                    # 从配置文件读取
                    config_files = [
                        '/etc/sing-box/config.json',
                        '/usr/local/etc/sing-box/config.json'
                    ]
                    for conf_dir in ['/etc/sing-box/conf']:
                        if os.path.exists(conf_dir):
                            for file in os.listdir(conf_dir):
                                if file.endswith('.json'):
                                    config_files.append(os.path.join(conf_dir, file))

                    for config_file in config_files:
                        if os.path.exists(config_file):
                            config_ports = self.parse_config_ports(config_file)
                            sb_ports.extend(config_ports)

                all_ports.extend(sb_ports)
                print(f"✅ 检测到Sing-box运行端口: {sb_ports}")
        except Exception as e:
            print(f"❌ 检测Sing-box失败: {e}")

        return list(set(all_ports))  # 去重

    async def send_notification(self, chat_id: int, message: str):
        """发送Telegram通知"""
        if not self.notification_enabled:
            return

        try:
            await self.application.bot.send_message(
                chat_id=chat_id,
                text=message,
                parse_mode='Markdown'
            )
        except Exception as e:
            logger.error(f"发送通知失败: {e}")

    async def start_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/start命令"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        welcome_msg = """
🤖 *端口监控机器人 - 终极安全版*

基于detect_ports_ultimate.sh功能构建

可用命令:
🔍 /status - 获取系统状态概览
📋 /ports - 检测所有服务端口
🔥 /firewall - 查看防火墙状态
🔒 /secure - 安全锁定防火墙
⚙️ /setup - 自动配置防火墙
📊 /monitor - 启动监控模式
📚 /help - 显示帮助信息

⚠️ 安全锁定功能将移除所有未知端口
        """
        await update.message.reply_text(welcome_msg, parse_mode='Markdown')

    async def status_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/status命令"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        timezone = self.get_timezone()
        current_time = datetime.now().strftime('%Y-%m-%d %H:%M:%S')

        status_msg = f"""
📊 *系统状态*

🕒 时区: {timezone}
🕐 当前时间: {current_time}
🏠 主机名: {os.uname().nodename}
🔥 防火墙: {self.firewall_manager.firewall_type}
🛡️ SSH端口: {self.get_ssh_port()}
        """
        await update.message.reply_text(status_msg, parse_mode='Markdown')

    async def ports_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/ports命令"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        await update.message.reply_text("🔍 正在检测端口，请稍候...")

        # 获取所有服务端口
        all_ports = self.get_all_service_ports()

        if not all_ports:
            await update.message.reply_text("❌ 未检测到任何服务端口")
            return

        status = "🔍 *端口检测结果*\n\n"
        status += f"📋 检测到端口: {', '.join(map(str, sorted(all_ports)))}\n\n"

        # 详细状态
        ssh_port = self.get_ssh_port()
        status += f"🛡️ SSH端口: {ssh_port}\n"

        # 进程状态
        for process_name in ['xray', 'sing-box']:
            try:
                result = subprocess.run(['pgrep', '-f', process_name],
                                      capture_output=True, text=True, timeout=5)
                if result.returncode == 0:
                    ports = self.get_process_ports(process_name)
                    if ports:
                        status += f"✅ {process_name.upper()}运行端口: {', '.join(map(str, ports))}\n"
                    else:
                        status += f"⚠️ {process_name.upper()}正在运行，但未检测到端口\n"
                else:
                    status += f"❌ {process_name.upper()}未运行\n"
            except Exception as e:
                status += f"❌ 检测{process_name.upper()}失败: {e}\n"

        await update.message.reply_text(status, parse_mode='Markdown')

    async def firewall_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/firewall命令"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        await update.message.reply_text("🔥 正在获取防火墙状态...")

        if self.firewall_manager.firewall_type == 'none':
            await update.message.reply_text("❌ 未检测到活跃防火墙")
            return

        status = f"🔥 *防火墙状态*\n\n"
        status += f"📋 防火墙类型: {self.firewall_manager.firewall_type}\n\n"

        try:
            if self.firewall_manager.firewall_type == 'firewalld':
                result = subprocess.run(['sudo', 'firewall-cmd', '--list-all'],
                                      capture_output=True, text=True, timeout=10)
                status += f"📋 当前配置:\n{result.stdout}"
            elif self.firewall_manager.firewall_type == 'ufw':
                result = subprocess.run(['sudo', 'ufw', 'status', 'verbose'],
                                      capture_output=True, text=True, timeout=10)
                status += f"📋 当前状态:\n{result.stdout}"

            await update.message.reply_text(status, parse_mode='Markdown')
        except Exception as e:
            await update.message.reply_text(f"❌ 获取防火墙状态失败: {e}")

    async def secure_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/secure命令 - 安全锁定防火墙"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        await update.message.reply_text("🔒 开始安全锁定防火墙...\n⚠️ 此操作将移除所有未知端口，5秒后开始...")

        # 获取所有服务端口
        allowed_ports = self.get_all_service_ports()

        if not allowed_ports:
            await update.message.reply_text("❌ 未检测到任何需要保留的端口")
            return

        # 发送确认消息
        ports_str = ', '.join(map(str, sorted(allowed_ports)))
        confirm_msg = f"""
🔒 *防火墙安全锁定*

将要保留的端口: `{ports_str}`
即将移除所有其他端口的访问规则。

此操作不可逆，确认继续吗？
        """
        await update.message.reply_text(confirm_msg, parse_mode='Markdown')

        # 延迟5秒执行
        await asyncio.sleep(5)

        if self.firewall_manager.reset_to_secure(allowed_ports):
            success_msg = f"""
✅ *防火墙安全锁定完成*

🔒 保留端口: `{ports_str}`
🔥 防火墙类型: {self.firewall_manager.firewall_type}
🕐 时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
            """
            await update.message.reply_text(success_msg, parse_mode='Markdown')

            # 通知管理员
            for chat_id in self.allowed_chat_ids:
                await self.send_notification(chat_id, success_msg)
        else:
            await update.message.reply_text("❌ 防火墙安全锁定失败")

    async def setup_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/setup命令 - 自动配置防火墙"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        if self.firewall_manager.firewall_type != 'none':
            await update.message.reply_text(f"✅ 防火墙已配置: {self.firewall_manager.firewall_type}")
            return

        await update.message.reply_text("🔧 正在自动配置防火墙...")

        firewall_type = self.firewall_manager.install_firewall()

        if firewall_type == 'none':
            await update.message.reply_text("❌ 防火墙配置失败")
        else:
            await update.message.reply_text(f"✅ 防火墙配置成功: {firewall_type}")

    async def monitor_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/monitor命令 - 监控模式"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        await update.message.reply_text("📊 监控模式已启动，每5分钟检查一次状态...")

        while True:
            try:
                # 监控逻辑
                ports = self.get_all_service_ports()
                firewall_status = self.firewall_manager.detect_firewall()

                if ports and firewall_status != 'none':
                    # 确保防火墙规则正确
                    for port in ports:
                        self.firewall_manager.add_rule(port, 'tcp')
                        self.firewall_manager.add_rule(port, 'udp')

                    status_msg = f"📊 监控状态正常\n🔥 防火墙: {firewall_status}\n📋 端口: {ports}"
                    await update.message.reply_text(status_msg)
                else:
                    await update.message.reply_text("⚠️ 监控异常，请检查系统状态")

                await asyncio.sleep(300)  # 5分钟

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"监控异常: {e}")
                await asyncio.sleep(60)

    async def help_command(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """处理/help命令"""
        if self.allowed_chat_ids and update.effective_chat.id not in self.allowed_chat_ids:
            await update.message.reply_text("❌ 未经授权的访问")
            return

        help_msg = """
📚 *帮助信息*

🤖 *端口监控机器人 - 终极安全版*

🔍 /status - 显示系统状态概览
📋 /ports - 检测并显示所有服务端口状态
🔥 /firewall - 显示防火墙配置和状态
🔒 /secure - 安全锁定防火墙（移除未知端口）
⚙️ /setup - 自动检测并配置防火墙
📊 /monitor - 启动实时监控模式
📚 /help - 显示此帮助信息

⚠️ *安全说明*:
- 安全锁定功能将移除所有非必需端口
- 自动配置功能会安装并配置防火墙
- 所有操作都需要管理员权限
- 建议先备份重要配置

基于detect_ports_ultimate.sh构建
        """
        await update.message.reply_text(help_msg, parse_mode='Markdown')

    def run(self):
        """启动机器人"""
        self.application = Application.builder().token(self.token).build()

        # 添加命令处理器
        self.application.add_handler(CommandHandler("start", self.start_command))
        self.application.add_handler(CommandHandler("status", self.status_command))
        self.application.add_handler(CommandHandler("ports", self.ports_command))
        self.application.add_handler(CommandHandler("firewall", self.firewall_command))
        self.application.add_handler(CommandHandler("secure", self.secure_command))
        self.application.add_handler(CommandHandler("setup", self.setup_command))
        self.application.add_handler(CommandHandler("monitor", self.monitor_command))
        self.application.add_handler(CommandHandler("help", self.help_command))

        # 启动机器人
        logger.info("机器人启动中...")
        self.application.run_polling()

def main():
    """主函数"""
    # 从环境变量获取配置
    token = os.getenv('TG_TOKEN')
    chat_ids_str = os.getenv('TG_CHAT_IDS', '')

    if not token:
        print("错误: 请设置TG_TOKEN环境变量")
        sys.exit(1)

    allowed_chat_ids = []
    if chat_ids_str:
        try:
            allowed_chat_ids = [int(cid.strip()) for cid in chat_ids_str.split(',') if cid.strip()]
        except ValueError:
            print("错误: TG_CHAT_IDS格式不正确，应为逗号分隔的数字")
            sys.exit(1)

    bot = PortMonitorBot(token, allowed_chat_ids)
    bot.run()

if __name__ == '__main__':
    main()