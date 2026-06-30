package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/vision"
)

var (
	WarmUpOCR                   = vision.WarmUpOCR
	LogMatProfile               = vision.LogMatProfile
	GetTessClient               = vision.GetTessClient
	CloseTessClient             = vision.CloseTessClient
	SaveMetaImage               = vision.SaveMetaImage
	SaveMetaImageLocked         = vision.SaveMetaImageLocked
	ImageToMatToImagePreprocess = vision.ImageToMatToImagePreprocess
	ConfigureNativeAllocator    = vision.ConfigureNativeAllocator
)

type PreprocessOptions = vision.PreprocessOptions

func macroUsesOCR(m *models.Macro) bool {
	return vision.MacroUsesOCR(m)
}
