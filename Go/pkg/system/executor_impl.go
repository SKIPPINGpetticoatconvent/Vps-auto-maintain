package system

import (
	"os/exec"
	"strconv"
	"strings"
	"time"
)

type RealSystemExecutor struct{}

func NewRealSystemExecutor() *RealSystemExecutor {
	return &RealSystemExecutor{}
}

func (e *RealSystemExecutor) IsInstalled(program string) bool {
	// Check typical paths
	paths := []string{
		"/usr/local/bin/" + program,
		"/usr/bin/" + program,
		"/bin/" + program,
	}

	for _, p := range paths {
		if _, err := exec.LookPath(p); err == nil {
			return true
		}
	}

	// Fallback to searching in PATH
	if _, err := exec.LookPath(program); err == nil {
		return true
	}

	return false
}

func (e *RealSystemExecutor) GetSystemTime() (time.Time, string) {
	now := time.Now()
	name, _ := now.Zone()
	return now, name
}

func (e *RealSystemExecutor) RunCommand(cmd string, args ...string) (string, error) {
	out, err := exec.Command(cmd, args...).CombinedOutput()
	output := strings.TrimSpace(string(out))
	if err != nil {
		return output, err
	}
	return output, nil
}

func (e *RealSystemExecutor) RunCoreMaintain() (string, error) {
	return e.RunCommand("/usr/local/bin/vps-maintain-core.sh")
}

func (e *RealSystemExecutor) RunRulesMaintain() (string, error) {
	return e.RunCommand("/usr/local/bin/vps-maintain-rules.sh")
}

func (e *RealSystemExecutor) Reboot() error {
	_, err := e.RunCommand("reboot")
	return err
}

func (e *RealSystemExecutor) GetLogs(lines int) (string, error) {
	return e.RunCommand("journalctl", "-u", "vps-tg-bot", "-n", strconv.Itoa(lines), "--no-pager")
}
