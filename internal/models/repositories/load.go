package repositories

import "sync"

// LoadAll initializes macro and program repositories in parallel after config is loaded.
func LoadAll() {
	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		MacroRepo()
		wg.Done()
	}()
	go func() {
		ProgramRepo()
		wg.Done()
	}()
	wg.Wait()
}
