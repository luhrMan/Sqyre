package utils

import "log"

func HandleError(e error, t, f func()) {
	log.Println(e)
	switch e != nil {
	case true:
		t()
	case false:
		f()
	}
}
