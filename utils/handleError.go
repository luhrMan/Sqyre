package utils

import "log"

func HandleError(e error, t, f func()) {
	switch e != nil {
	case true:
		log.Println(e)
		t()
	case false:
		f()
	}
}
