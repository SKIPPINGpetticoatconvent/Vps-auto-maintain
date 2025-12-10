# VPS Telegram Bot (Go Port) Specification

## 1. Overview
This project ports the functionality of the existing Python-based VPS Telegram Bot to Go. The bot provides an interactive interface via Telegram to manage VPS maintenance tasks, view system status, and schedule automated updates.

**Key Constraints:**
- **Language:** Go (Golang)
- **Persistence:** Scheduled tasks must survive bot restarts.
- **Security:** No hardcoded secrets. Token and Chat ID must be loaded from environment variables or a config file.
- **Architecture:** Modular design with TDD anchors.

## 2. Architecture & Modules

The application is divided into the following core modules:

1.  **Config:** Handles configuration loading (Env vars, Flags).
2.  **System:** Abstraction for OS-level operations (Command execution, File checks, Reboot, Logs).
3.  **Scheduler:** Manages cron jobs with persistence (Save/Load state).
4.  **Bot:** Handles Telegram interactions (Updates, Menus, Callbacks).

### 2.1 Module: Config (`pkg/config`)

**Responsibility:** Load application configuration.

**Fields:**
- `TelegramToken` (string): From `TG_TOKEN` env or flag.
- `AdminChatID` (int64): From `TG_CHAT_ID` env or flag.
- `StateFile` (string): Path to JSON file for persisting scheduler state (default: `state.json`).

**TDD Anchors:**
- `TestLoadConfig_EnvVars`: Verify loading from environment variables.
- `TestLoadConfig_Flags`: Verify loading from command-line flags.
- `TestLoadConfig_Validation`: Verify error when required fields are missing.

### 2.2 Module: System (`pkg/system`)

**Responsibility:** Execute shell commands and query system state.

**Interfaces:**
```go
type SystemExecutor interface {
    // Checks if a binary exists in PATH or specific locations
    IsInstalled(program string) bool
    
    // Returns current system time and timezone
    GetSystemTime() (time.Time, string)
    
    // Executes a shell command and returns output/error
    RunCommand(cmd string, args ...string) (string, error)
    
    // Specific maintenance wrappers (calling the shell scripts created by installer)
    RunCoreMaintain() (string, error)
    RunRulesMaintain() (string, error)
    
    // System operations
    Reboot() error
    GetLogs(lines int) (string, error)
}
```

**Pseudocode:**
```go
// IsInstalled checks /usr/local/bin and PATH
FUNCTION IsInstalled(program):
    IF FileExists("/usr/local/bin/" + program) RETURN true
    IF Exec("which", program) succeeds RETURN true
    RETURN false

// RunCoreMaintain calls the legacy shell script
FUNCTION RunCoreMaintain():
    RETURN RunCommand("/usr/local/bin/vps-maintain-core.sh")

// GetLogs fetches journalctl logs
FUNCTION GetLogs(lines):
    RETURN RunCommand("journalctl", "-u", "vps-tg-bot", "-n", lines, "--no-pager")
```

**TDD Anchors:**
- `TestIsInstalled_Mock`: Mock file system check.
- `TestRunCommand_Success`: Verify command execution output.
- `TestGetLogs_Format`: Verify log command construction.

### 2.3 Module: Scheduler (`pkg/scheduler`)

**Responsibility:** Manage cron jobs and persist their schedule.

**Data Structure (Persistence):**
```json
{
  "core_maintain": "0 4 * * *",
  "rules_maintain": "0 7 * * 0"
}
```

**Interfaces:**
```go
type JobManager interface {
    Start()
    Stop()
    
    // Add or Update a job
    SetJob(name string, cronExp string, task func()) error
    
    // Remove a job
    RemoveJob(name string)
    
    // Clear all jobs
    ClearAll()
    
    // Get current job status for display
    GetJobStatus(name string) string // Returns "✅ Schedule" or "❌ Not Set"
    
    // Save/Load state
    SaveState() error
    LoadState() error
}
```

**Pseudocode:**
```go
FUNCTION SetJob(name, cronExp, task):
    IF jobs[name] exists:
        cron.Remove(jobs[name].EntryID)
    
    entryID = cron.AddFunc(cronExp, task)
    jobs[name] = {EntryID: entryID, Expression: cronExp}
    SaveState()

FUNCTION LoadState():
    data = ReadFile(StateFile)
    FOR name, cronExp IN data:
        // Re-register tasks based on name
        // Note: We need a registry of task functions (core vs rules)
        IF name == "core_maintain":
            SetJob(name, cronExp, CoreMaintainTask)
        ELSE IF name == "rules_maintain":
            SetJob(name, cronExp, RulesMaintainTask)
```

**TDD Anchors:**
- `TestSetJob_AddsEntry`: Verify job is added to internal map and cron.
- `TestPersistence_SaveLoad`: Verify state is written to and read from JSON.
- `TestClearAll`: Verify all jobs are removed.

### 2.4 Module: Bot (`pkg/bot`)

**Responsibility:** Handle Telegram UI logic.

**Key Components:**
- **Menu Handler:** Generates Inline Keyboards.
- **Callback Handler:** Routes button clicks to actions.
- **Auth Middleware:** Ensures only `AdminChatID` can interact.

**Menu Structure:**
- **Main Menu:**
    - 📊 System Status (`status`)
    - 🔧 Maintain Now (`maintain_now`)
    - ⚙️ Schedule Settings (`schedule_menu`)
    - 📋 View Logs (`view_logs`)
    - 🔄 Reboot VPS (`reboot_confirm`)

- **Maintain Menu:**
    - 🔧 Core Maintain (`maintain_core`)
    - 📜 Rules Update (`maintain_rules`)
    - 🔄 Full Maintain (`maintain_full`)
    - 🔙 Back (`back_main`)

- **Schedule Menu:**
    - ⏰ Set Core (Daily 04:00) (`schedule_core`)
    - 📅 Set Rules (Sun 07:00) (`schedule_rules`)
    - 🗑️ Clear All (`schedule_clear`)
    - 🔙 Back (`back_main`)

**Pseudocode:**
```go
FUNCTION HandleUpdate(update):
    IF update.Message != nil:
        IF update.Message.Text == "/start":
            ShowMainMenu(update.ChatID)
            
    IF update.CallbackQuery != nil:
        data = update.CallbackQuery.Data
        SWITCH data:
            CASE "status": ShowStatus()
            CASE "maintain_now": ShowMaintainMenu()
            CASE "maintain_core": ExecuteMaintain("core")
            CASE "schedule_core": Scheduler.SetJob("core_maintain", "0 4 * * *", TaskCore)
            // ... handle other cases
```

**TDD Anchors:**
- `TestHandleStart_Authorized`: Verify Main Menu is sent.
- `TestHandleStart_Unauthorized`: Verify access denied message.
- `TestCallback_Status`: Verify status message format.
- `TestCallback_Schedule`: Verify scheduler is called.

## 3. Implementation Steps

1.  **Setup:** Initialize Go module structure (already done).
2.  **Config:** Implement `pkg/config`.
3.  **System:** Implement `pkg/system` with real and mock implementations.
4.  **Scheduler:** Implement `pkg/scheduler` with persistence logic.
5.  **Bot Logic:** Implement `pkg/bot` connecting all pieces.
6.  **Integration:** Wire everything in `main.go`.

## 4. Environment Variables

- `TG_TOKEN`: Telegram Bot Token (Required)
- `TG_CHAT_ID`: Admin Chat ID (Required)
- `STATE_FILE`: Path to schedule state file (Optional, default `./state.json`)
