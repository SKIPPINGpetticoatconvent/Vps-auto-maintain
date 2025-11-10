# 配置说明

## 配置方式

程序支持两种配置方式：

### 1. 环境变量（推荐）

在运行程序前设置环境变量：

```bash
export TG_TOKEN="your_bot_token"
export TG_CHAT_ID="your_chat_id"
export CORE_SCRIPT="/usr/local/bin/vps-maintain-core.sh"  # 可选
export RULES_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"  # 可选

./vps-tg-bot
```

### 2. 交互式输入

如果环境变量未设置，且程序在交互式终端中运行，程序会自动提示用户输入配置信息。

```bash
./vps-tg-bot
```

程序会依次提示：
1. Telegram Bot Token（隐藏输入）
2. Telegram Chat ID（管理员）
3. 核心维护脚本路径（可选，有默认值）
4. 规则更新脚本路径（可选，有默认值）

## 获取配置信息

### Telegram Bot Token

1. 在 Telegram 中搜索 [@BotFather](https://t.me/BotFather)
2. 发送 `/newbot` 创建新 Bot
3. 按照提示设置 Bot 名称和用户名
4. BotFather 会返回 Bot Token，格式类似：`123456789:ABCdefGHIjklMNOpqrsTUVwxyz`

### Telegram Chat ID

1. 在 Telegram 中搜索 [@userinfobot](https://t.me/userinfobot)
2. 发送任意消息，Bot 会返回你的 Chat ID
3. 或者使用 [@getidsbot](https://t.me/getidsbot) 获取

## systemd 服务配置

如果使用 systemd 服务运行，建议在服务文件中设置环境变量：

```ini
[Service]
Environment="TG_TOKEN=your_bot_token"
Environment="TG_CHAT_ID=your_chat_id"
Environment="CORE_SCRIPT=/usr/local/bin/vps-maintain-core.sh"
Environment="RULES_SCRIPT=/usr/local/bin/vps-maintain-rules.sh"
```

## 安全建议

1. **不要将 Token 提交到版本控制系统**
   - 使用环境变量或配置文件（不提交到 Git）
   - 使用 `.env` 文件（添加到 `.gitignore`）

2. **使用最小权限原则**
   - 仅授予必要的权限
   - 定期更换 Token

3. **保护配置文件**
   - 设置适当的文件权限：`chmod 600 .env`
   - 不要在日志中输出敏感信息

## 配置文件示例

### 使用 .env 文件（需要额外支持）

如果使用 `godotenv` 等库，可以创建 `.env` 文件：

```bash
TG_TOKEN=your_bot_token
TG_CHAT_ID=your_chat_id
CORE_SCRIPT=/usr/local/bin/vps-maintain-core.sh
RULES_SCRIPT=/usr/local/bin/vps-maintain-rules.sh
```

### systemd 服务文件示例

```ini
[Unit]
Description=VPS Telegram Bot Management System (Go)
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/vps-tg-bot
Environment="TG_TOKEN=your_bot_token"
Environment="TG_CHAT_ID=your_chat_id"
Environment="CORE_SCRIPT=/usr/local/bin/vps-maintain-core.sh"
Environment="RULES_SCRIPT=/usr/local/bin/vps-maintain-rules.sh"
ExecStart=/opt/vps-tg-bot/vps-tg-bot
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## 故障排查

### 问题：环境变量未设置，但程序没有提示输入

**原因**：程序检测到不在交互式终端中运行（如 systemd 服务）

**解决**：
1. 在 systemd 服务文件中设置环境变量
2. 或者手动运行程序（不在后台运行）

### 问题：交互式输入时 Token 显示在屏幕上

**原因**：终端不支持隐藏输入

**解决**：使用环境变量方式配置

### 问题：Chat ID 格式错误

**原因**：Chat ID 必须是数字

**解决**：确保输入的是纯数字，不包含其他字符

## 验证配置

配置完成后，程序启动时会：
1. 验证 Token 有效性
2. 尝试连接 Telegram API
3. 如果成功，会显示 Bot 用户名

如果配置错误，程序会显示相应的错误信息。
