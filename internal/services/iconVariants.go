package services

import (
	"Squire/internal/config"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// IconVariantService provides filesystem operations for discovering, validating,
// and managing icon variant files.
type IconVariantService struct{
	basePath string // Optional base path for testing, overrides config.ImagesPath
}

// NewIconVariantService creates a new IconVariantService instance.
func NewIconVariantService() *IconVariantService {
	return &IconVariantService{}
}

// GetVariants returns all variant names for an item by scanning the filesystem.
// It discovers variants by looking for files matching the pattern "{ItemName}|*.png".
// Returns a sorted list of variant names for consistent UI display.
func (s *IconVariantService) GetVariants(programName, itemName string) ([]string, error) {
	if programName == "" || itemName == "" {
		return nil, fmt.Errorf("program name and item name cannot be empty")
	}

	iconsPath := s.getIconsPath(programName)
	
	// Check if the icons directory exists
	if _, err := os.Stat(iconsPath); os.IsNotExist(err) {
		return []string{}, nil
	}

	// Pattern to match: {ItemName}|*.png
	pattern := filepath.Join(iconsPath, itemName+config.ProgramDelimiter+"*"+config.PNG)
	
	matches, err := filepath.Glob(pattern)
	if err != nil {
		return nil, fmt.Errorf("failed to search for variants: %w", err)
	}

	// Also check for non-variant icon (legacy single icon without delimiter)
	legacyPath := filepath.Join(iconsPath, itemName+config.PNG)
	if _, err := os.Stat(legacyPath); err == nil {
		// Legacy icon exists, treat it as a variant with empty name
		matches = append(matches, legacyPath)
	}

	// Extract variant names from file paths
	variants := make([]string, 0, len(matches))
	for _, match := range matches {
		filename := filepath.Base(match)
		// Remove .png extension
		nameWithoutExt := strings.TrimSuffix(filename, config.PNG)
		
		// Check if it contains the delimiter
		if strings.Contains(nameWithoutExt, config.ProgramDelimiter) {
			// Extract variant name (text after delimiter)
			parts := strings.SplitN(nameWithoutExt, config.ProgramDelimiter, 2)
			if len(parts) == 2 {
				variants = append(variants, parts[1])
			}
		} else {
			// Legacy icon without delimiter - use empty string as variant name
			variants = append(variants, "")
		}
	}

	// Sort for consistent UI display
	sort.Strings(variants)

	return variants, nil
}

// GetVariantPath constructs the full path to a variant icon file.
// If variantName is empty, returns the path to the legacy non-variant icon.
func (s *IconVariantService) GetVariantPath(programName, itemName, variantName string) string {
	iconsPath := s.getIconsPath(programName)
	
	if variantName == "" {
		// Legacy icon without variant
		return filepath.Join(iconsPath, itemName+config.PNG)
	}
	
	// Variant icon with delimiter
	filename := itemName + config.ProgramDelimiter + variantName + config.PNG
	return filepath.Join(iconsPath, filename)
}

// AddVariant copies a file to the icons directory with proper naming convention.
// It validates the source file, creates necessary directories, and copies the file
// with the format "{ItemName}|{VariantName}.png".
func (s *IconVariantService) AddVariant(programName, itemName, variantName, sourcePath string) error {
	if programName == "" || itemName == "" || variantName == "" {
		return fmt.Errorf("program name, item name, and variant name cannot be empty")
	}

	if sourcePath == "" {
		return fmt.Errorf("source path cannot be empty")
	}

	// Validate source file
	if err := s.ValidateVariantFile(sourcePath); err != nil {
		return fmt.Errorf("invalid source file: %w", err)
	}

	// Sanitize variant name to prevent path traversal
	if strings.Contains(variantName, "..") || strings.Contains(variantName, "/") || strings.Contains(variantName, "\\") {
		return fmt.Errorf("invalid variant name: contains path separators")
	}
	variantName = filepath.Base(variantName)

	// Check if variant already exists
	existingVariants, err := s.GetVariants(programName, itemName)
	if err != nil {
		return fmt.Errorf("failed to check existing variants: %w", err)
	}

	for _, existing := range existingVariants {
		if existing == variantName {
			return fmt.Errorf("variant '%s' already exists for item '%s'", variantName, itemName)
		}
	}

	// Check variant count limit (max 100 per requirements)
	if len(existingVariants) >= 100 {
		return fmt.Errorf("maximum variant limit (100) reached for item '%s'", itemName)
	}

	// Create icons directory if it doesn't exist
	iconsPath := s.getIconsPath(programName)
	if err := os.MkdirAll(iconsPath, 0755); err != nil {
		return fmt.Errorf("failed to create icons directory: %w", err)
	}

	// Construct destination path
	destPath := s.GetVariantPath(programName, itemName, variantName)

	// Copy file
	if err := s.copyFile(sourcePath, destPath); err != nil {
		return fmt.Errorf("failed to copy file: %w", err)
	}

	return nil
}

// DeleteVariant removes a variant icon file from the filesystem.
func (s *IconVariantService) DeleteVariant(programName, itemName, variantName string) error {
	if programName == "" || itemName == "" {
		return fmt.Errorf("program name and item name cannot be empty")
	}

	// Get the file path
	filePath := s.GetVariantPath(programName, itemName, variantName)

	// Check if file exists
	if _, err := os.Stat(filePath); os.IsNotExist(err) {
		// File doesn't exist - idempotent operation, not an error
		return nil
	}

	// Delete the file
	if err := os.Remove(filePath); err != nil {
		return fmt.Errorf("failed to delete variant file: %w", err)
	}

	return nil
}

// GetBaseItemName extracts the base name from a full item name by parsing
// text before the ProgramDelimiter. If no delimiter is found, returns the
// full name unchanged.
func (s *IconVariantService) GetBaseItemName(fullItemName string) string {
	if !strings.Contains(fullItemName, config.ProgramDelimiter) {
		return fullItemName
	}

	parts := strings.SplitN(fullItemName, config.ProgramDelimiter, 2)
	return parts[0]
}

// ValidateVariantFile checks if a file exists and is a valid PNG by verifying
// the PNG file header signature.
func (s *IconVariantService) ValidateVariantFile(path string) error {
	if path == "" {
		return fmt.Errorf("file path cannot be empty")
	}

	// Clean the path to prevent traversal
	path = filepath.Clean(path)

	// Check if file exists
	info, err := os.Stat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("file does not exist")
		}
		return fmt.Errorf("failed to access file: %w", err)
	}

	// Check if it's a regular file
	if !info.Mode().IsRegular() {
		return fmt.Errorf("path is not a regular file")
	}

	// Open file to check PNG header
	file, err := os.Open(path)
	if err != nil {
		return fmt.Errorf("failed to open file: %w", err)
	}
	defer file.Close()

	// Read first 8 bytes for PNG signature
	header := make([]byte, 8)
	n, err := file.Read(header)
	if err != nil {
		return fmt.Errorf("failed to read file header: %w", err)
	}

	if n < 8 {
		return fmt.Errorf("file too small to be a valid PNG")
	}

	// PNG signature: \x89PNG\r\n\x1a\n
	pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
	for i := 0; i < 8; i++ {
		if header[i] != pngSignature[i] {
			return fmt.Errorf("file is not a valid PNG (invalid header signature)")
		}
	}

	return nil
}

// getIconsPath returns the icons directory path for a program.
// This method can be overridden in tests by using a custom basePath.
func (s *IconVariantService) getIconsPath(programName string) string {
	if s.basePath != "" {
		return filepath.Join(s.basePath, programName) + "/"
	}
	return config.ImagesPath + "icons/" + programName + "/"
}

// copyFile copies a file from src to dst.
func (s *IconVariantService) copyFile(src, dst string) error {
	sourceFile, err := os.Open(src)
	if err != nil {
		return err
	}
	defer sourceFile.Close()

	destFile, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer destFile.Close()

	_, err = io.Copy(destFile, sourceFile)
	if err != nil {
		return err
	}

	// Sync to ensure data is written to disk
	return destFile.Sync()
}
