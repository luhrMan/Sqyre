package models

import "testing"

func TestCellRect_singleCell(t *testing.T) {
	// 100x100 area, 2x2 grid → each cell 50x50
	lx, ty, rx, by, err := CellRect(0, 0, 100, 100, 2, 2, CellRange{1, 1, 1, 1})
	if err != nil {
		t.Fatal(err)
	}
	if lx != 0 || ty != 0 || rx != 50 || by != 50 {
		t.Fatalf("got %d,%d-%d,%d want 0,0-50,50", lx, ty, rx, by)
	}

	lx, ty, rx, by, err = CellRect(0, 0, 100, 100, 2, 2, CellRange{2, 2, 2, 2})
	if err != nil {
		t.Fatal(err)
	}
	if lx != 50 || ty != 50 || rx != 100 || by != 100 {
		t.Fatalf("got %d,%d-%d,%d want 50,50-100,100", lx, ty, rx, by)
	}
}

func TestCellRect_multiCellUnion(t *testing.T) {
	lx, ty, rx, by, err := CellRect(10, 20, 110, 120, 2, 2, CellRange{1, 1, 2, 2})
	if err != nil {
		t.Fatal(err)
	}
	if lx != 10 || ty != 20 || rx != 110 || by != 120 {
		t.Fatalf("full grid got %d,%d-%d,%d", lx, ty, rx, by)
	}

	// Top-left 1x2 (row1 cols 1-2) with swapped corners
	lx, ty, rx, by, err = CellRect(0, 0, 100, 100, 2, 2, CellRange{1, 2, 1, 1})
	if err != nil {
		t.Fatal(err)
	}
	if lx != 0 || ty != 0 || rx != 100 || by != 50 {
		t.Fatalf("got %d,%d-%d,%d want 0,0-100,50", lx, ty, rx, by)
	}
}

func TestCellCenter(t *testing.T) {
	x, y, err := CellCenter(0, 0, 100, 100, 2, 2, CellRange{1, 1, 1, 1})
	if err != nil {
		t.Fatal(err)
	}
	if x != 25 || y != 25 {
		t.Fatalf("center = %d,%d want 25,25", x, y)
	}
}

func TestValidateCellRange_outOfBounds(t *testing.T) {
	if err := ValidateCellRange(2, 2, CellRange{1, 1, 3, 1}); err == nil {
		t.Fatal("expected error")
	}
	if err := ValidateCellRange(0, 2, CellRange{1, 1, 1, 1}); err == nil {
		t.Fatal("expected error for zero rows")
	}
}
