package uiutil

import (
	"strings"

	"Sqyre/internal/config"
	"Sqyre/internal/services"
)

// IconPathForTarget returns the Fyne resource path for a target "ProgramName|baseName".
// Returns empty string if the target format is invalid or no variant is found.
func IconPathForTarget(target string) string {
	programName, baseName, ok := strings.Cut(target, config.ProgramDelimiter)
	if !ok || programName == "" || baseName == "" {
		return ""
	}
	iconService := services.IconVariantServiceInstance()
	variants, err := iconService.GetVariants(programName, baseName)
	if err != nil || len(variants) == 0 {
		return ""
	}
	var selectedVariant string
	for _, v := range variants {
		if v == "Original" {
			selectedVariant = v
			break
		}
	}
	if selectedVariant == "" {
		selectedVariant = variants[0]
	}
	return programName + config.ProgramDelimiter + baseName + config.ProgramDelimiter + selectedVariant + config.PNG
}
