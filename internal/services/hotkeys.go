package services

import (
	"Squire/internal/models"
	"log"
	"os"
	"slices"
	"strings"

	// hook "github.com/robotn/gohook"
	hook "github.com/luhrMan/gohook"
)

func FailsafeHotkey() {
	fs := []string{"esc", "ctrl", "shift"}
	hook.Register(hook.KeyDown, fs, func(e hook.Event) {
		log.Println("FAILSAFE INITIATED: EXITING PROGRAM...")
		os.Exit(0)
	})
}

func StartHook() {
	log.Println("hook started")
	s := hook.Start()
	<-hook.Process(s)
}

func ParseMacroHotkey(hk string) []string {
	if hk == "" {
		return []string{}
	}
	parts := strings.Split(hk, "+")

	for i, part := range parts {
		parts[i] = strings.TrimSpace(part)
	}
	return parts
}

func ReverseParseMacroHotkey(hk []string) string {
	var str string
	for i, k := range hk {
		if i == 0 {
			str = k
			continue
		}
		str = str + " + " + k
	}
	return str
}

func RegisterHotkey(hk []string, cb func(e hook.Event)) {
	if slices.Equal(hk, []string{}) {
		log.Println("do not register empty hotkeys!")
		return
	}
	log.Printf("registering hotkey %v", hk)
	hook.Register(hook.KeyDown, hk, cb)
}
func UnregisterHotkey(hk []string) {
	log.Println("unregistering hotkey:", hk)
	hook.Unregister(hook.KeyDown, hk)
}

func MacroHotkeyCallback(m *models.Macro) func(e hook.Event) {
	return func(e hook.Event) {
		log.Printf("pressed %v, executing %v", m.Hotkey, m.Name)
		Execute(m.Root, m)
	}
}

// add this to hook.go file in robotgo hook (I did add this as a self-maintained fork... fuc)
// Unregister removes a previously registered hook event handler
// It takes the same parameters as Register to identify which hook to remove
// func Unregister(when uint8, cmds []string) bool {
// 	lck.Lock()
// 	defer lck.Unlock()

// 	targetKeys := []uint16{}
// 	for _, v := range cmds {
// 		targetKeys = append(targetKeys, Keycode[v])
// 	}

// 	if eventKeys, ok := events[when]; ok {
// 		for i, keyIndex := range eventKeys {
// 			if equalKeySlices(keys[keyIndex], targetKeys) {
// 				events[when] = append(eventKeys[:i], eventKeys[i+1:]...)

// 				delete(keys, keyIndex)
// 				delete(cbs, keyIndex)

// 				for j, usedKey := range used {
// 					if usedKey == keyIndex {
// 						used[j] = -1
// 						// used = append(used[:j], used[j+1:]...)
// 						break
// 					}
// 				}
// 				return true
// 			}
// 		}
// 	}

// 	return false
// }

// equalKeySlices compares two slices of uint16 for equality
// func equalKeySlices(a, b []uint16) bool {
// 	if len(a) != len(b) {
// 		return false
// 	}

// 	// Create maps to count occurrences of each key
// 	mapA := make(map[uint16]int)
// 	mapB := make(map[uint16]int)

// 	for _, k := range a {
// 		mapA[k]++
// 	}

// 	for _, k := range b {
// 		mapB[k]++
// 	}

// 	// Compare maps
// 	for k, v := range mapA {
// 		if mapB[k] != v {
// 			return false
// 		}
// 	}

// 	for k, v := range mapB {
// 		if mapA[k] != v {
// 			return false
// 		}
// 	}

// 	return true
// }
