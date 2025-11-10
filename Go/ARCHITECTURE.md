# 架构设计文档

## 概述

本项目采用模块化设计，将 Telegram Bot 逻辑、系统命令封装和定时任务分离，便于维护和扩展。

## 模块结构

### 1. 配置模块 (`pkg/config`)

负责加载和管理配置信息。

- **Config**: 配置结构体
  - `Token`: Telegram Bot Token
  - `AdminChatID`: 管理员 Chat ID
  - `CoreScript`: 核心维护脚本路径
  - `RulesScript`: 规则更新脚本路径

- **Load()**: 从环境变量加载配置

### 2. 系统命令模块 (`pkg/system`)

封装系统命令执行，提供统一的接口。

#### 2.1 命令执行器 (`executor.go`)

- **CommandExecutor**: 命令执行器
  - 支持超时控制
  - 支持 Shell/Bash 命令执行
  - 自动错误处理

- **主要方法**:
  - `Execute()`: 执行命令（带超时）
  - `ExecuteShell()`: 执行 Shell 命令
  - `ExecuteBash()`: 执行 Bash 命令
  - `CheckCommandExists()`: 检查命令是否存在

#### 2.2 系统操作 (`actions.go`)

- **CheckUptime()**: 检查系统运行时间
- **GetDetailedStatus()**: 获取详细系统状态（内存、磁盘、CPU等）
- **RunMaintenance()**: 执行系统维护脚本
- **RunRulesMaintenance()**: 执行规则更新脚本
- **RebootVPS()**: 重启 VPS
- **ShutdownVPS()**: 关闭 VPS
- **GetLogs()**: 获取服务日志

### 3. Telegram Bot 模块 (`pkg/bot`)

#### 3.1 Bot 核心 (`handler.go`)

- **Bot**: Bot 主结构体
  - 管理 Telegram API 连接
  - 处理消息更新
  - 提供消息发送接口

- **主要方法**:
  - `NewBot()`: 创建 Bot 实例
  - `Start()`: 启动 Bot
  - `SendMessage()`: 发送消息给管理员
  - `SendMessageToChat()`: 发送消息到指定聊天
  - `IsAdmin()`: 检查管理员权限
  - `ShowMainMenu()`: 显示主菜单
  - `ExecuteMaintenance()`: 执行维护
  - `ExecuteReboot()`: 执行重启

#### 3.2 命令路由 (`router.go`)

- **Router**: 命令路由器
  - 统一管理命令和回调处理
  - 支持动态注册处理器
  - 统一的错误处理

- **主要方法**:
  - `RegisterCommand()`: 注册命令处理器
  - `RegisterCallback()`: 注册回调处理器
  - `HandleMessage()`: 处理消息
  - `HandleCallback()`: 处理回调

- **已注册命令**:
  - `/start`: 显示主菜单
  - `/status`: 查看系统状态
  - `/maintain`: 执行维护
  - `/reboot`: 重启 VPS
  - `/help`: 显示帮助

- **已注册回调**:
  - `status`: 系统状态
  - `status_detail`: 详细状态
  - `maintain_core`: 执行维护
  - `logs`: 查看日志
  - `reboot`: 重启 VPS
  - `back`: 返回主菜单

### 4. 定时任务模块 (`pkg/scheduler`)

- **Scheduler**: 定时任务调度器
  - 基于 cron 表达式
  - 支持多任务管理
  - 自动发送通知

- **主要方法**:
  - `NewScheduler()`: 创建调度器
  - `Start()`: 启动调度器（默认每周日 04:00 执行维护）
  - `Stop()`: 停止调度器
  - `AddTask()`: 添加自定义任务
  - `GetTasks()`: 获取任务列表

- **默认任务**:
  - 每周日 04:00 执行系统维护
  - 自动执行规则更新
  - 维护完成后自动重启

## 数据流

```
用户消息/回调
    ↓
Router (路由分发)
    ↓
CommandHandler / CallbackHandler
    ↓
Bot (执行操作)
    ↓
System (系统命令封装)
    ↓
CommandExecutor (执行命令)
    ↓
返回结果给用户
```

## 特性

### 1. 命令执行

- ✅ 超时控制（防止命令卡死）
- ✅ 错误处理（统一的错误处理机制）
- ✅ 日志记录（记录所有操作）
- ✅ 结果缓存（读取脚本输出文件）

### 2. Bot 功能

- ✅ 权限验证（仅管理员可访问）
- ✅ 命令路由（统一的路由管理）
- ✅ 错误处理（友好的错误提示）
- ✅ 交互式菜单（内联键盘）

### 3. 定时任务

- ✅ Cron 表达式支持
- ✅ 多任务管理
- ✅ 自动通知
- ✅ 任务列表查询

## 扩展指南

### 添加新命令

1. 在 `router.go` 中注册命令：
```go
r.RegisterCommand("newcmd", r.handleNewCommand)
```

2. 实现处理函数：
```go
func (r *Router) handleNewCommand(message *tgbotapi.Message) error {
    // 处理逻辑
    return nil
}
```

### 添加新系统命令

1. 在 `actions.go` 中添加函数：
```go
func NewSystemCommand() (string, error) {
    ctx := context.Background()
    output, err := defaultExecutor.ExecuteShell(ctx, "your command")
    return output, err
}
```

### 添加新定时任务

```go
scheduler.AddTask("0 0 12 * * *", func() {
    // 每天中午12点执行
})
```

## 最佳实践

1. **错误处理**: 所有函数都应返回错误，并在调用处处理
2. **超时控制**: 长时间运行的命令应设置超时
3. **日志记录**: 重要操作应记录日志
4. **权限验证**: 所有用户操作都应验证权限
5. **资源清理**: 使用 defer 确保资源正确释放

## 性能优化

- 使用 goroutine 处理异步操作（如重启）
- 命令执行器支持超时，防止资源泄漏
- 日志长度限制，避免消息过长
- 结果缓存，减少重复执行

## 安全考虑

- 仅管理员可访问 Bot
- 命令执行前验证脚本存在
- 超时控制防止恶意命令
- 错误信息不泄露敏感信息
