package controllers

import (
	"fyne.io/fyne/v2/data/binding"
)

type ActionBindings struct {
	boundBaseAction     binding.Struct
	boundAdvancedAction binding.Struct

	boundWait  binding.Struct
	boundKey   binding.Struct
	boundMove  binding.Struct
	boundClick binding.Struct

	boundLoop        binding.Struct
	boundImageSearch binding.Struct
	boundOcr         binding.Struct

	boundSearchArea binding.Struct
	boundPoint      binding.Struct
}

type ProgramBindings struct {
	boundProgram binding.Struct
}

func BindProgram() {

}
func Set() {

}

func Delete() {}
