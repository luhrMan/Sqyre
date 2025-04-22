package utils

import (
	"log"
	"os"
	"slices"
	"strings"

	hook "github.com/robotn/gohook"
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
	// hook.Register(hook.KeyDown, hk, func(e hook.Event) {
	// 	log.Printf("pressed %v, executing %v", hk, m.Name)
	// 	m.ExecuteActionTree()
	// })
}
func UnregisterHotkey(hk []string) {
	log.Println("unregistering hotkey:", hk)
	hook.Unregister(hook.KeyDown, hk)
}
