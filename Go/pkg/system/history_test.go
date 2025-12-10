package system

import (
	"os"
	"testing"
	"time"
)

func TestFileHistoryRecorder_AddAndGetRecords(t *testing.T) {
	filePath := "test_history.json"
	defer os.Remove(filePath)

	recorder := NewFileHistoryRecorder(filePath)

	record := &MaintainHistoryRecord{
		ID:        "1",
		Type:      "test",
		StartTime: time.Now(),
		EndTime:   time.Now().Add(time.Minute),
		Status:    "success",
		Result:    "ok",
	}

	if err := recorder.AddRecord(record); err != nil {
		t.Fatalf("AddRecord failed: %v", err)
	}

	records, err := recorder.GetRecords(10)
	if err != nil {
		t.Fatalf("GetRecords failed: %v", err)
	}

	if len(records) != 1 {
		t.Errorf("Expected 1 record, got %d", len(records))
	}

	if records[0].ID != record.ID {
		t.Errorf("Expected record ID %s, got %s", record.ID, records[0].ID)
	}
}

func TestFileHistoryRecorder_Limit(t *testing.T) {
	filePath := "test_history_limit.json"
	defer os.Remove(filePath)

	recorder := NewFileHistoryRecorder(filePath)

	// Add 105 records
	for i := 0; i < 105; i++ {
		record := &MaintainHistoryRecord{
			ID:        "id",
			Type:      "test",
			StartTime: time.Now(),
			EndTime:   time.Now(),
			Status:    "success",
		}
		if err := recorder.AddRecord(record); err != nil {
			t.Fatalf("AddRecord failed: %v", err)
		}
	}

	records, err := recorder.GetRecords(200)
	if err != nil {
		t.Fatalf("GetRecords failed: %v", err)
	}

	if len(records) > 100 {
		t.Errorf("Expected max 100 records, got %d", len(records))
	}
}