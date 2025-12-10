package system

import "time"

type SystemExecutor interface {
	IsInstalled(program string) bool
	GetSystemTime() (time.Time, string)
	RunCommand(cmd string, args ...string) (string, error)
	RunCoreMaintain() (string, error)
	RunRulesMaintain() (string, error)
	Reboot() error
	GetLogs(lines int) (string, error)
}
