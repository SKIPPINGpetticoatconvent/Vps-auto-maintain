package system

import (
	"encoding/json"
	"fmt"
	"os"
	"sync"
	"time"
)

// MaintainHistoryRecord 维护历史记录
type MaintainHistoryRecord struct {
	ID        string    `json:"id"`
	Type      string    `json:"type"`
	StartTime time.Time `json:"start_time"`
	EndTime   time.Time `json:"end_time"`
	Status    string    `json:"status"` // "success", "failed"
	Result    string    `json:"result"`
	Error     string    `json:"error,omitempty"`
}

// HistoryRecorder 历史记录器接口
type HistoryRecorder interface {
	AddRecord(record *MaintainHistoryRecord) error
	GetRecords(limit int) ([]*MaintainHistoryRecord, error)
}

// FileHistoryRecorder 基于文件的历史记录器
type FileHistoryRecorder struct {
	filePath string
	mutex    sync.Mutex
}

// NewFileHistoryRecorder 创建新的文件历史记录器
func NewFileHistoryRecorder(filePath string) *FileHistoryRecorder {
	return &FileHistoryRecorder{
		filePath: filePath,
	}
}

// AddRecord 添加记录
func (r *FileHistoryRecorder) AddRecord(record *MaintainHistoryRecord) error {
	r.mutex.Lock()
	defer r.mutex.Unlock()

	records, err := r.loadRecords()
	if err != nil {
		return err
	}

	// 添加新记录到开头
	records = append([]*MaintainHistoryRecord{record}, records...)

	// 限制记录数量，例如保留最近100条
	if len(records) > 100 {
		records = records[:100]
	}

	return r.saveRecords(records)
}

// GetRecords 获取记录
func (r *FileHistoryRecorder) GetRecords(limit int) ([]*MaintainHistoryRecord, error) {
	r.mutex.Lock()
	defer r.mutex.Unlock()

	records, err := r.loadRecords()
	if err != nil {
		return nil, err
	}

	if limit > 0 && len(records) > limit {
		return records[:limit], nil
	}

	return records, nil
}

func (r *FileHistoryRecorder) loadRecords() ([]*MaintainHistoryRecord, error) {
	if _, err := os.Stat(r.filePath); os.IsNotExist(err) {
		return []*MaintainHistoryRecord{}, nil
	}

	data, err := os.ReadFile(r.filePath)
	if err != nil {
		return nil, fmt.Errorf("读取历史记录文件失败: %v", err)
	}

	var records []*MaintainHistoryRecord
	if err := json.Unmarshal(data, &records); err != nil {
		return nil, fmt.Errorf("解析历史记录失败: %v", err)
	}

	return records, nil
}

func (r *FileHistoryRecorder) saveRecords(records []*MaintainHistoryRecord) error {
	data, err := json.MarshalIndent(records, "", "  ")
	if err != nil {
		return fmt.Errorf("序列化历史记录失败: %v", err)
	}

	if err := os.WriteFile(r.filePath, data, 0600); err != nil {
		return fmt.Errorf("写入历史记录文件失败: %v", err)
	}

	return nil
}