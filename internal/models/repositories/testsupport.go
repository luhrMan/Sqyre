package repositories

import "sync"

// ResetAllForTesting clears repository singletons so tests can reinitialize config.
func ResetAllForTesting() {
	macroRepo = nil
	macroOnce = sync.Once{}
	programRepo = nil
	programOnce = sync.Once{}
}
