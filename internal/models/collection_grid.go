package models

import "fmt"

// CellRange is a 1-based inclusive rectangle of collection cells.
type CellRange struct {
	R1, C1 int
	R2, C2 int
}

// Normalize returns a range with min/max corners ordered (still 1-based).
func (r CellRange) Normalize() CellRange {
	out := r
	if out.R1 > out.R2 {
		out.R1, out.R2 = out.R2, out.R1
	}
	if out.C1 > out.C2 {
		out.C1, out.C2 = out.C2, out.C1
	}
	return out
}

// ValidateCellRange checks that the range is within [1..rows]×[1..cols].
func ValidateCellRange(rows, cols int, r CellRange) error {
	if rows < 1 || cols < 1 {
		return fmt.Errorf("collection grid %dx%d: rows and cols must be >= 1", rows, cols)
	}
	r = r.Normalize()
	if r.R1 < 1 || r.C1 < 1 || r.R2 > rows || r.C2 > cols {
		return fmt.Errorf("cell range %d,%d-%d,%d out of bounds for %dx%d grid", r.R1, r.C1, r.R2, r.C2, rows, cols)
	}
	return nil
}

// CellRect returns the axis-aligned union rectangle of the selected cells
// within area bounds (leftX, topY, rightX, bottomY). Indices are 1-based inclusive.
func CellRect(leftX, topY, rightX, bottomY, rows, cols int, r CellRange) (lx, ty, rx, by int, err error) {
	if err = ValidateCellRange(rows, cols, r); err != nil {
		return 0, 0, 0, 0, err
	}
	r = r.Normalize()
	width := rightX - leftX
	height := bottomY - topY
	if width <= 0 || height <= 0 {
		return 0, 0, 0, 0, fmt.Errorf("invalid search area bounds %d,%d-%d,%d", leftX, topY, rightX, bottomY)
	}

	// Integer division: cell i occupies [floor((i-1)*size/n), floor(i*size/n)).
	cellLeft := leftX + (r.C1-1)*width/cols
	cellRight := leftX + r.C2*width/cols
	cellTop := topY + (r.R1-1)*height/rows
	cellBottom := topY + r.R2*height/rows
	return cellLeft, cellTop, cellRight, cellBottom, nil
}

// CellCenter returns the center of the union rectangle of the selected cells.
func CellCenter(leftX, topY, rightX, bottomY, rows, cols int, r CellRange) (x, y int, err error) {
	lx, ty, rx, by, err := CellRect(leftX, topY, rightX, bottomY, rows, cols, r)
	if err != nil {
		return 0, 0, err
	}
	return (lx + rx) / 2, (ty + by) / 2, nil
}
