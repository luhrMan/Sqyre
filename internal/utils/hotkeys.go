package utils

import (
	"log"
	"os"
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
