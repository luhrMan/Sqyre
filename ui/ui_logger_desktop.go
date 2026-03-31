//go:build !js

package ui

import (
	"path/filepath"

	"Sqyre/internal/config"
	"Sqyre/internal/logger"
)

func setupUILogger() {
	logger.SetLogFile(filepath.Join(config.GetSqyreDir(), "sqyre.log"))
}
