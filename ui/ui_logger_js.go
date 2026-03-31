//go:build js

package ui

import "Sqyre/internal/logger"

func setupUILogger() {
	logger.SetLogFile("")
}
