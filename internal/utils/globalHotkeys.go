package utils

import (
	"log"
	"os"

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
	s := hook.Start()
	// defer hook.End()
	// defer log.Println("hook ended")
	log.Println("Hook started")
	<-hook.Process(s)
}
