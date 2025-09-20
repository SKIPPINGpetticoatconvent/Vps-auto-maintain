@echo off
REM ---------------------------------------------------------------------------------
REM Telegram端口监控机器人启动脚本 (Windows版本)
REM 基于detect_ports_ultimate.sh的Python版本
REM ---------------------------------------------------------------------------------

setlocal enabledelayedexpansion

echo.
echo 🤖 Telegram端口监控机器人启动器 (Windows)
echo ============================================================
echo.

REM 检查Python是否安装
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ 错误: 未找到Python，请先安装Python 3.7+
    echo 下载地址: https://www.python.org/downloads/
    pause
    exit /b 1
)

REM 检查pip是否安装
pip --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ 错误: 未找到pip，请先安装pip
    pause
    exit /b 1
)

REM 创建虚拟环境
if not exist "venv" (
    echo 🔧 创建Python虚拟环境...
    python -m venv venv
    if !errorlevel! neq 0 (
        echo ❌ 错误: 虚拟环境创建失败
        pause
        exit /b 1
    )
    echo ✅ 虚拟环境创建成功
) else (
    echo ⚠️ 虚拟环境已存在，跳过创建
)

REM 激活虚拟环境并安装依赖
echo 📦 激活虚拟环境并安装依赖...
call venv\Scripts\activate.bat
if !errorlevel! neq 0 (
    echo ❌ 错误: 虚拟环境激活失败
    pause
    exit /b 1
)

REM 升级pip
python -m pip install --upgrade pip >nul 2>&1

REM 安装依赖
if exist "requirements.txt" (
    echo 正在安装依赖...
    pip install -r requirements.txt
    if !errorlevel! neq 0 (
        echo ❌ 错误: 依赖安装失败
        pause
        exit /b 1
    )
    echo ✅ 依赖安装成功
) else (
    echo ❌ 错误: 未找到requirements.txt文件
    pause
    exit /b 1
)

REM 创建日志目录
if not exist "logs" (
    mkdir logs
    echo ✅ 日志目录创建成功
) else (
    echo ⚠️ 日志目录已存在
)

REM 检查配置文件
if not exist "config.json" (
    echo ❌ 错误: 未找到配置文件config.json
    echo 请确保config.json文件存在并配置正确
    pause
    exit /b 1
)

REM 设置环境变量
echo 🔧 检查环境变量...
set TG_TOKEN=
set TG_CHAT_IDS=

REM 从环境变量读取配置
if "%TG_TOKEN%"=="" (
    echo ⚠️ 警告: 未设置TG_TOKEN环境变量
    echo 请设置环境变量或检查config.json文件
)

if "%TG_CHAT_IDS%"=="" (
    echo ⚠️ 警告: 未设置TG_CHAT_IDS环境变量
    echo 请设置环境变量或检查config.json文件
)

echo.
echo 🚀 启动机器人...
echo ============================================================
echo.

REM 启动机器人
python start_bot.py

REM 保持窗口打开以查看错误信息
echo.
echo ============================================================
if %errorlevel% neq 0 (
    echo ❌ 机器人启动失败 (错误码: %errorlevel%)
    echo 请检查上方错误信息
) else (
    echo ✅ 机器人已停止
)
echo ============================================================
echo 按任意键关闭窗口...
pause >nul