package custom_widgets

import (
	"log"

	"fyne.io/fyne/v2/data/binding"
)

func CustomStringToBool(str binding.String, koc string, dl binding.DataListener) binding.Bool { //koc == key or click
	tmp := binding.NewString()

	str.AddListener(binding.NewDataListener(func() {
		val, err := str.Get()
		if err != nil {
			log.Println(err)
			return
		}

		switch val {
		case "right", "Right", "down", "Down":
			tmp.Set("true")
		case "left", "Left", "up", "Up":
			tmp.Set("false")
		default:
			tmp.Set(val)
		}
	}))
	tmp.AddListener(binding.NewDataListener(func() {
		val, err := tmp.Get()
		if err != nil {
			log.Println(err)
			return
		}
		switch val {
		case "true":
			if koc == "click" {
				str.Set("right")
			} else if koc == "key" {
				str.Set("down")
			}
		case "false":
			if koc == "click" {
				str.Set("left")
			} else if koc == "key" {
				str.Set("up")
			}
		default:
			str.Set(val)
		}
	}))
	tmp.AddListener(dl)
	str.AddListener(dl)

	return binding.StringToBool(tmp)
}
