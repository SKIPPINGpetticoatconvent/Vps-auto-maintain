# Git 提交与发布规范

## 1. 提交消息 (Conventional Commits)

要求 AI 在建议提交信息时遵循以下格式：

- `feat:` 新功能
- `fix:` 修复 Bug
- `test:` 增加测试
- `docs:` 修改文档

## 2. 版本管理

- 每次修改 `pkg/` 或 `src/` 核心逻辑后，提醒用户是否需要更新 `README.md` 中的版本号。
- 当用户询问“如何发布”时，优先检查 `go-build.yml` 或 `rust-ci.yml` 配置文件。
