package actions

import (
	"Sqyre/internal/config"
	"strings"
)

// CoordinateRef references program-owned coordinate data (search area or point) by key.
// Format: "programName~entityName" (same delimiter as image search item targets).
type CoordinateRef string

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

func (r CoordinateRef) IsEmpty() bool {
	return r == ""
}

func (r CoordinateRef) String() string {
	return string(r)
}

// Program returns the program name portion of the ref, or "" for legacy name-only refs.
func (r CoordinateRef) Program() string {
	parts := strings.SplitN(string(r), config.ProgramDelimiter, 2)
	if len(parts) == 2 {
		return parts[0]
	}
	return ""
}

// Name returns the search area or point name portion of the ref.
func (r CoordinateRef) Name() string {
	parts := strings.SplitN(string(r), config.ProgramDelimiter, 2)
	if len(parts) == 2 {
		return parts[1]
	}
	return string(r)
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
