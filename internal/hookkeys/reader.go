package hookkeys

// Reader reports physical keys currently held using platform-native input state.
type Reader interface {
	PressedKeyNames() []string
	Close()
}

// ChordFullyReleased reports whether none of the named keys are currently held.
func ChordFullyReleased(r Reader, names []string) bool {
	if r == nil {
		return true
	}
	pressed := r.PressedKeyNames()
	for _, want := range names {
		for _, have := range pressed {
			if have == want {
				return false
			}
		}
	}
	return true
}

// ChordAllPressed reports whether every named key is currently held.
func ChordAllPressed(r Reader, names []string) bool {
	if r == nil || len(names) == 0 {
		return false
	}
	pressed := r.PressedKeyNames()
	for _, want := range names {
		found := false
		for _, have := range pressed {
			if have == want {
				found = true
				break
			}
		}
		if !found {
			return false
		}
	}
	return true
}
