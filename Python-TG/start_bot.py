#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Telegram端口监控机器人启动脚本
"""

import os
import sys
import json
from pathlib import Path

def load_config():
    """加载配置文件"""
    config_file = Path(__file__).parent / "config.json"
    if not config_file.exists():
        print("❌ 配置文件不存在: config.json")
        sys.exit(1)

    try:
        with open(config_file, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception as e:
        print(f"❌ 配置文件加载失败: {e}")
        sys.exit(1)

def check_dependencies():
    """检查依赖项"""
    try:
        import telegram
        print("✅ python-telegram-bot 已安装")
    except ImportError:
        print("❌ python-telegram-bot 未安装，请运行: pip install -r requirements.txt")
        return False

    try:
        import dotenv
        print("✅ python-dotenv 已安装")
    except ImportError:
        print("❌ python-dotenv 未安装，请运行: pip install -r requirements.txt")
        return False

    return True

def setup_logging(config):
    """设置日志系统"""
    log_config = config.get('logging', {})
    log_level = log_config.get('level', 'INFO')
    log_file = log_config.get('file', 'logs/bot.log')
    log_to_console = log_config.get('log_to_console', True)

    # 创建日志目录
    log_dir = Path(log_file).parent
    log_dir.mkdir(exist_ok=True)

    # 设置日志
    import logging
    from logging.handlers import RotatingFileHandler

    logger = logging.getLogger()
    logger.setLevel(getattr(logging, log_level))

    # 文件处理器
    file_handler = RotatingFileHandler(
        log_file,
        maxBytes=log_config.get('max_size', 10485760),
        backupCount=log_config.get('backup_count', 5)
    )
    file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)

    # 控制台处理器
    if log_to_console:
        console_handler = logging.StreamHandler()
        console_formatter = logging.Formatter('%(levelname)s - %(message)s')
        console_handler.setFormatter(console_formatter)
        logger.addHandler(console_handler)

    return logger

def main():
    """主函数"""
    print("🤖 Telegram端口监控机器人启动器")
    print("=" * 50)

    # 加载配置
    config = load_config()
    print("✅ 配置文件加载成功")

    # 检查依赖
    if not check_dependencies():
        sys.exit(1)

    # 设置日志
    logger = setup_logging(config)
    logger.info("机器人启动器初始化")

    # 导入主机器人类
    try:
        from tg_port_monitor import PortMonitorBot
        logger.info("主模块导入成功")
    except ImportError as e:
        logger.error(f"主模块导入失败: {e}")
        sys.exit(1)

    # 获取Telegram配置
    tg_config = config.get('telegram', {})
    token = os.getenv('TG_TOKEN') or tg_config.get('token')
    chat_ids_str = os.getenv('TG_CHAT_IDS', ','.join(map(str, tg_config.get('allowed_chat_ids', []))))

    if not token:
        logger.error("未设置TG_TOKEN环境变量或配置文件中")
        print("❌ 错误: 请设置TG_TOKEN环境变量或检查配置文件")
        sys.exit(1)

    # 解析聊天ID
    allowed_chat_ids = []
    if chat_ids_str:
        try:
            allowed_chat_ids = [int(cid.strip()) for cid in chat_ids_str.split(',') if cid.strip()]
        except ValueError:
            logger.error("TG_CHAT_IDS格式不正确")
            print("❌ 错误: TG_CHAT_IDS格式不正确，应为逗号分隔的数字")
            sys.exit(1)

    if not allowed_chat_ids:
        logger.error("未设置TG_CHAT_IDS")
        print("❌ 错误: 请设置TG_CHAT_IDS环境变量或检查配置文件")
        sys.exit(1)

    # 创建机器人实例
    print("✅ 机器人配置验证成功")
    logger.info(f"机器人配置: 允许的聊天ID: {allowed_chat_ids}")

    try:
        bot = PortMonitorBot(token, allowed_chat_ids)
        print("🚀 机器人启动中...")
        logger.info("机器人启动")
        bot.run()
    except KeyboardInterrupt:
        print("\n🛑 机器人已停止")
        logger.info("机器人被用户停止")
    except Exception as e:
        logger.error(f"机器人运行时发生错误: {e}")
        print(f"❌ 机器人运行失败: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()