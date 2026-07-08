package editor

import (
	"Sqyre/internal/models/repositories"
	"sort"
	"sync"
)

var programTagsCache struct {
	mu        sync.Mutex
	byProgram map[string][]string
}

// InvalidateProgramTagsCache drops cached tag lists for programName, or all programs when empty.
func InvalidateProgramTagsCache(programName string) {
	programTagsCache.mu.Lock()
	defer programTagsCache.mu.Unlock()
	if programTagsCache.byProgram == nil {
		return
	}
	if programName == "" {
		programTagsCache.byProgram = make(map[string][]string)
		return
	}
	delete(programTagsCache.byProgram, programName)
}

// ResetProgramTagsCacheForTesting clears the program tag cache (tests only).
func ResetProgramTagsCacheForTesting() {
	InvalidateProgramTagsCache("")
}

func collectProgramTagsFromRepo(programName string) []string {
	if programName == "" {
		return nil
	}
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		return nil
	}
	tagMap := make(map[string]bool)
	for _, itemName := range ProgramItemRepo(program).GetAllKeys() {
		item, err := ProgramItemRepo(program).Get(itemName)
		if err != nil {
			continue
		}
		for _, tag := range item.Tags {
			tagMap[tag] = true
		}
	}
	tags := make([]string, 0, len(tagMap))
	for tag := range tagMap {
		tags = append(tags, tag)
	}
	sort.Strings(tags)
	return tags
}
