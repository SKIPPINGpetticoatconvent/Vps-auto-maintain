# Telegram端口监控机器人 - 终极安全版

基于`detect_ports_ultimate.sh`的Python版本，实现Telegram交互式端口监控和防火墙安全管理。

## 🚀 功能特性

### 核心功能
- 🤖 **Telegram交互界面** - 通过Telegram机器人进行远程管理
- 🔍 **实时端口检测** - 自动检测Xray、Sing-box、SSH等服务端口
- 🔥 **防火墙自动管理** - 自动检测、安装和配置UFW/Firewalld
- 🔒 **安全锁定功能** - 移除未知端口，实现最小化攻击面
- 📊 **系统状态监控** - 实时监控系统状态和防火墙配置
- 📱 **Telegram通知** - 重要事件自动通知管理员

### 高级特性
- ⚙️ **自动配置** - 无人值守的防火墙自动配置
- 📝 **完整日志** - 详细的操作日志记录
- 🔄 **进程监控** - 实时监控服务进程状态
- 🌐 **多平台支持** - 支持Linux/Windows系统
- 🛡️ **安全防护** - 内置多种安全机制

## 📋 系统要求

- **Python** 3.7+
- **Linux/Windows** 操作系统
- **Telegram机器人令牌**
- **管理员权限** (用于防火墙操作)

## 🛠️ 安装部署

### 方法一：自动部署（推荐）

#### Linux系统
```bash
# 克隆项目
git clone <repository-url>
cd Python-TG

# 运行部署脚本
chmod +x deploy.sh
./deploy.sh
```

#### Windows系统
```batch
# 克隆项目
git clone <repository-url>
cd Python-TG

# 运行启动脚本
start_bot.bat
```

### 方法二：手动安装

1. **创建虚拟环境**
   ```bash
   python3 -m venv venv
   source venv/bin/activate  # Linux
   # 或
   venv\Scripts\activate     # Windows
   ```

2. **安装依赖**
   ```bash
   pip install -r requirements.txt
   ```

3. **配置机器人**
   - 编辑 `config.json` 文件
   - 设置Telegram机器人令牌和聊天ID

4. **启动机器人**
   ```bash
   python start_bot.py
   ```

## ⚙️ 配置说明

### config.json 配置文件

```json
{
  "telegram": {
    "token": "你的机器人令牌",
    "allowed_chat_ids": [你的聊天ID],
    "admin_chat_ids": [管理员聊天ID],
    "notification_enabled": true
  },
  "monitoring": {
    "check_interval": 300,
    "alert_on_changes": true,
    "services": {
      "xray": {
        "enabled": true,
        "process_name": "xray",
        "config_paths": ["/etc/xray/config.json"]
      },
      "sing_box": {
        "enabled": true,
        "process_name": "sing-box",
        "config_paths": ["/etc/sing-box/config.json"]
      }
    }
  },
  "firewall": {
    "auto_configure": true,
    "allowed_ports": [22, 80, 443],
    "auto_clean_unknown": true
  },
  "security": {
    "secure_lock_enabled": true,
    "auto_secure_on_startup": false,
    "whitelist_ports": [22]
  },
  "logging": {
    "level": "INFO",
    "file": "logs/bot.log",
    "log_to_console": true
  }
}
```

### 环境变量配置

```bash
export TG_TOKEN="你的机器人令牌"
export TG_CHAT_IDS="123456789,987654321"
```

## 📱 使用方法

### 启动机器人后，发送以下命令：

- `/start` - 开始使用机器人
- `/status` - 查看系统状态概览
- `/ports` - 检测所有服务端口
- `/firewall` - 查看防火墙状态
- `/secure` - 安全锁定防火墙
- `/setup` - 自动配置防火墙
- `/monitor` - 启动监控模式
- `/help` - 显示帮助信息

### 安全锁定示例

1. 发送 `/secure` 命令
2. 机器人会显示将要保留的端口列表
3. 确认后自动清理未知端口
4. 完成安全锁定并发送通知

## 🔐 安全说明

### 重要提醒
- ⚠️ **安全锁定功能会移除所有未知端口**，请谨慎使用
- 🔑 **建议先备份重要配置**再进行安全锁定
- 👤 **只有授权用户**才能执行敏感操作
- 📝 **所有操作都有详细日志**记录

### 推荐配置
- 启用防火墙自动配置
- 设置合理的监控间隔
- 配置多个管理员账号
- 定期检查日志文件

## 📊 监控功能

### 自动监控
- 每5分钟检查一次系统状态
- 检测服务进程运行状态
- 监控防火墙配置变化
- 异常情况自动通知

### 手动检查
- 实时端口扫描
- 防火墙规则验证
- 服务状态检查
- 系统资源监控

## 🚨 故障排除

### 常见问题

1. **机器人无响应**
   - 检查网络连接
   - 验证Telegram令牌
   - 查看日志文件

2. **防火墙操作失败**
   - 确认管理员权限
   - 检查防火墙服务状态
   - 查看系统日志

3. **端口检测不准确**
   - 检查服务进程名
   - 验证配置文件路径
   - 查看系统端口状态

### 日志文件
- 主日志: `logs/bot.log`
- 系统日志: `/var/log/syslog` (Linux)
- 事件日志: Windows事件查看器 (Windows)

### 获取帮助
- 查看 `/help` 命令
- 检查日志文件
- 联系技术支持

## 🔄 更新升级

### 自动更新
```bash
git pull origin main
pip install -r requirements.txt --upgrade
```

### 手动更新
1. 备份配置文件
2. 替换程序文件
3. 重新安装依赖
4. 重启机器人服务

## 📝 许可证

本项目基于MIT许可证开源。

## 🙏 致谢

本项目基于 `detect_ports_ultimate.sh` 脚本的功能实现，感谢原作者的贡献。

---

**注意**: 请在使用前仔细阅读安全说明，确保理解所有功能后再进行操作。