package actions

import (
	"Sqyre/internal/config"
	"fmt"
	"strconv"
	"strings"
)

// CoordinateRef references program-owned coordinate data (search area, point, or collection cell range).
//
// Formats:
//   - "programName~entityName" — point or search area
//   - "programName~collectionName@r1,c1-r2,c2" — collection cell selection (1-based inclusive)
//   - "entityName" — legacy name-only
type CoordinateRef string

const collectionCellSep = "@"

// NewCoordinateRef builds a ref from a program name and entity name.
func NewCoordinateRef(program, name string) CoordinateRef {
	if name == "" {
		return ""
	}
	if program == "" {
		return CoordinateRef(name)
	}
	return CoordinateRef(program + config.ProgramDelimiter + name)
}

// NewCollectionRef builds a collection cell-range ref (1-based inclusive).
// Corners are normalized so r1<=r2 and c1<=c2.
func NewCollectionRef(program, name string, r1, c1, r2, c2 int) CoordinateRef {
	if r1 > r2 {
		r1, r2 = r2, r1
	}
	if c1 > c2 {
		c1, c2 = c2, c1
	}
	base := NewCoordinateRef(program, name)
	if base.IsEmpty() {
		return ""
	}
	return CoordinateRef(fmt.Sprintf("%s%s%d,%d-%d,%d", base, collectionCellSep, r1, c1, r2, c2))
}

func (r CoordinateRef) IsEmpty() bool {
	return r == ""
}

func (r CoordinateRef) String() string {
	return string(r)
}

// IsCollection reports whether this ref includes a cell-range suffix.
func (r CoordinateRef) IsCollection() bool {
	_, _, _, _, ok := r.CellRange()
	return ok
}

// CellRange parses the @r1,c1-r2,c2 suffix (1-based inclusive, normalized).
// ok is false when absent or malformed.
func (r CoordinateRef) CellRange() (r1, c1, r2, c2 int, ok bool) {
	_, suffix, cut := strings.Cut(string(r), collectionCellSep)
	if !cut || suffix == "" {
		return 0, 0, 0, 0, false
	}
	start, end, cut := strings.Cut(suffix, "-")
	if !cut {
		return 0, 0, 0, 0, false
	}
	r1, c1, ok = parseCellPair(start)
	if !ok {
		return 0, 0, 0, 0, false
	}
	r2, c2, ok = parseCellPair(end)
	if !ok {
		return 0, 0, 0, 0, false
	}
	if r1 > r2 {
		r1, r2 = r2, r1
	}
	if c1 > c2 {
		c1, c2 = c2, c1
	}
	return r1, c1, r2, c2, true
}

func parseCellPair(s string) (row, col int, ok bool) {
	a, b, cut := strings.Cut(s, ",")
	if !cut {
		return 0, 0, false
	}
	row, err1 := strconv.Atoi(strings.TrimSpace(a))
	col, err2 := strconv.Atoi(strings.TrimSpace(b))
	if err1 != nil || err2 != nil {
		return 0, 0, false
	}
	return row, col, true
}

// entityPart returns the program~name or name portion without a cell-range suffix.
func (r CoordinateRef) entityPart() string {
	base, _, _ := strings.Cut(string(r), collectionCellSep)
	return base
}

// Program returns the program name portion of the ref, or "" for legacy name-only refs.
func (r CoordinateRef) Program() string {
	parts := strings.SplitN(r.entityPart(), config.ProgramDelimiter, 2)
	if len(parts) == 2 {
		return parts[0]
	}
	return ""
}

// Name returns the entity name portion of the ref (without cell-range suffix).
func (r CoordinateRef) Name() string {
	parts := strings.SplitN(r.entityPart(), config.ProgramDelimiter, 2)
	if len(parts) == 2 {
		return parts[1]
	}
	return r.entityPart()
}

// DisplayLabel returns a short label for UI display.
func (r CoordinateRef) DisplayLabel() string {
	if r.IsEmpty() {
		return ""
	}
	if p := r.Program(); p != "" {
		return string(r)
	}
	return r.Name()
}

// WithEntityName returns a copy of the ref with the entity name replaced,
// preserving any collection cell-range suffix.
func (r CoordinateRef) WithEntityName(program, newName string) CoordinateRef {
	if r1, c1, r2, c2, ok := r.CellRange(); ok {
		return NewCollectionRef(program, newName, r1, c1, r2, c2)
	}
	return NewCoordinateRef(program, newName)
}
