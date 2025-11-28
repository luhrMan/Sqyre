package services

import (
	"Squire/internal/config"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/lithammer/fuzzysearch/fuzzy"
)

// MaskInfo represents information about a mask file
type MaskInfo struct {
	Name    string // filename without extension
	Path    string // full file path
	Program string // parent directory name
	Format  string // file extension (.png, .jpg, .jpeg)
}

// MaskDiscoveryService provides filesystem operations for discovering and managing mask files
type MaskDiscoveryService struct {
	basePath string // Optional base path for testing, overrides config.GetMasksPath()
}

var maskDiscoveryServiceInstance *MaskDiscoveryService

// MaskDiscoveryServiceInstance returns the singleton instance of MaskDiscoveryService
func MaskDiscoveryServiceInstance() *MaskDiscoveryService {
	if maskDiscoveryServiceInstance == nil {
		maskDiscoveryServiceInstance = &MaskDiscoveryService{}
	}
	return maskDiscoveryServiceInstance
}

// NewMaskDiscoveryService creates a new MaskDiscoveryService instance
// Deprecated: Use MaskDiscoveryServiceInstance() for the singleton instance
func NewMaskDiscoveryService() *MaskDiscoveryService {
	return &MaskDiscoveryService{}
}

// ScanMasksDirectory scans the masks directory structure and returns mask information organized by program
func (s *MaskDiscoveryService) ScanMasksDirectory() (map[string][]MaskInfo, error) {
	masksPath := s.getMasksPath()
	
	// Check if masks directory exists and is accessible
	if _, err := os.Stat(masksPath); err != nil {
		if os.IsNotExist(err) {
			// Return empty map if directory doesn't exist
			return make(map[string][]MaskInfo), nil
		}
		if os.IsPermission(err) {
			return nil, fmt.Errorf("masks directory is not accessible due to insufficient permissions: %s", masksPath)
		}
		return nil, fmt.Errorf("failed to access masks directory %s: %w", masksPath, err)
	}
	
	// Read program directories with enhanced error handling
	programDirs, err := os.ReadDir(masksPath)
	if err != nil {
		if os.IsPermission(err) {
			return nil, fmt.Errorf("insufficient permissions to read masks directory: %s", masksPath)
		}
		return nil, fmt.Errorf("failed to read masks directory %s: %w", masksPath, err)
	}
	
	masksByProgram := make(map[string][]MaskInfo)
	var scanErrors []string
	
	for _, programDir := range programDirs {
		if !programDir.IsDir() {
			continue // Skip non-directory files
		}
		
		programName := programDir.Name()
		programPath := filepath.Join(masksPath, programName)
		
		// Scan masks in this program directory with error collection
		masks, err := s.scanProgramMasks(programPath, programName)
		if err != nil {
			// Collect error but continue with other programs
			scanErrors = append(scanErrors, fmt.Sprintf("program '%s': %v", programName, err))
			continue
		}
		
		// Only add program if it has masks
		if len(masks) > 0 {
			masksByProgram[programName] = masks
		}
	}
	
	// Log accumulated scan errors for debugging
	if len(scanErrors) > 0 {
		for _, errMsg := range scanErrors {
			fmt.Printf("Warning: Error scanning masks for %s\n", errMsg)
		}
	}
	
	return masksByProgram, nil
}

// scanProgramMasks scans a specific program directory for mask files
func (s *MaskDiscoveryService) scanProgramMasks(programPath, programName string) ([]MaskInfo, error) {
	// Check if program directory is accessible
	if _, err := os.Stat(programPath); err != nil {
		if os.IsNotExist(err) {
			return []MaskInfo{}, nil // Return empty slice if directory doesn't exist
		}
		if os.IsPermission(err) {
			return nil, fmt.Errorf("insufficient permissions to access program directory: %s", programPath)
		}
		return nil, fmt.Errorf("failed to access program directory %s: %w", programPath, err)
	}
	
	files, err := os.ReadDir(programPath)
	if err != nil {
		if os.IsPermission(err) {
			return nil, fmt.Errorf("insufficient permissions to read program directory: %s", programPath)
		}
		return nil, fmt.Errorf("failed to read program directory %s: %w", programPath, err)
	}
	
	var masks []MaskInfo
	var fileErrors []string
	supportedExtensions := []string{config.PNG, ".jpg", ".jpeg"}
	
	for _, file := range files {
		if file.IsDir() {
			continue // Skip subdirectories
		}
		
		filename := file.Name()
		ext := strings.ToLower(filepath.Ext(filename))
		
		// Check if file has supported extension
		if !s.isSupportedFormat(ext, supportedExtensions) {
			continue
		}
		
		// Validate file accessibility
		filePath := filepath.Join(programPath, filename)
		if _, err := os.Stat(filePath); err != nil {
			if os.IsPermission(err) {
				fileErrors = append(fileErrors, fmt.Sprintf("insufficient permissions for file '%s'", filename))
			} else {
				fileErrors = append(fileErrors, fmt.Sprintf("cannot access file '%s': %v", filename, err))
			}
			continue
		}
		
		// Create mask info
		maskInfo := MaskInfo{
			Name:    strings.TrimSuffix(filename, ext), // Remove extension
			Path:    filePath,
			Program: programName,
			Format:  ext,
		}
		
		masks = append(masks, maskInfo)
	}
	
	// Log file access errors for debugging
	if len(fileErrors) > 0 {
		for _, errMsg := range fileErrors {
			fmt.Printf("Warning: %s in program '%s'\n", errMsg, programName)
		}
	}
	
	// Sort masks by name for consistent display
	sort.Slice(masks, func(i, j int) bool {
		return masks[i].Name < masks[j].Name
	})
	
	return masks, nil
}

// GetMasksForProgram returns all masks for a specific program
func (s *MaskDiscoveryService) GetMasksForProgram(programName string) ([]MaskInfo, error) {
	if programName == "" {
		return nil, fmt.Errorf("program name cannot be empty")
	}
	
	programPath := filepath.Join(s.getMasksPath(), programName)
	
	// Check if program directory exists
	if _, err := os.Stat(programPath); os.IsNotExist(err) {
		return []MaskInfo{}, nil // Return empty slice if directory doesn't exist
	}
	
	return s.scanProgramMasks(programPath, programName)
}

// FilterMasks filters a slice of masks by name using fuzzy matching for consistency with other tabs
func (s *MaskDiscoveryService) FilterMasks(masks []MaskInfo, searchTerm string) []MaskInfo {
	if searchTerm == "" {
		return masks // Return all masks if no search term
	}
	
	var filtered []MaskInfo
	searchTerm = strings.ToLower(searchTerm)
	
	for _, mask := range masks {
		if fuzzy.MatchFold(searchTerm, mask.Name) {
			filtered = append(filtered, mask)
		}
	}
	
	return filtered
}

// ValidateMaskFile checks if a file exists and has a supported image format
func (s *MaskDiscoveryService) ValidateMaskFile(path string) error {
	if path == "" {
		return fmt.Errorf("file path cannot be empty")
	}
	
	// Clean the path to prevent traversal
	path = filepath.Clean(path)
	
	// Check if file exists and is accessible
	info, err := os.Stat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("mask file does not exist: %s", path)
		}
		if os.IsPermission(err) {
			return fmt.Errorf("insufficient permissions to access mask file: %s", path)
		}
		return fmt.Errorf("failed to access mask file %s: %w", path, err)
	}
	
	// Check if it's a regular file
	if !info.Mode().IsRegular() {
		return fmt.Errorf("path is not a regular file: %s", path)
	}
	
	// Check file size (prevent loading extremely large files)
	const maxFileSize = 50 * 1024 * 1024 // 50MB limit
	if info.Size() > maxFileSize {
		return fmt.Errorf("mask file too large (%d MB, maximum: %d MB): %s", 
			info.Size()/(1024*1024), maxFileSize/(1024*1024), path)
	}
	
	// Check if file is readable
	file, err := os.Open(path)
	if err != nil {
		if os.IsPermission(err) {
			return fmt.Errorf("insufficient permissions to read mask file: %s", path)
		}
		return fmt.Errorf("cannot open mask file %s: %w", path, err)
	}
	file.Close()
	
	// Check file extension
	ext := strings.ToLower(filepath.Ext(path))
	supportedExtensions := []string{config.PNG, ".jpg", ".jpeg"}
	
	if !s.isSupportedFormat(ext, supportedExtensions) {
		return fmt.Errorf("unsupported file format: %s (supported: PNG, JPG, JPEG)", ext)
	}
	
	return nil
}

// isSupportedFormat checks if the given extension is in the list of supported formats
func (s *MaskDiscoveryService) isSupportedFormat(ext string, supportedExtensions []string) bool {
	for _, supported := range supportedExtensions {
		if ext == supported {
			return true
		}
	}
	return false
}

// getMasksPath returns the masks directory path
// This method can be overridden in tests by using a custom basePath
func (s *MaskDiscoveryService) getMasksPath() string {
	if s.basePath != "" {
		return s.basePath
	}
	return config.GetMasksPath()
}

// GetProgramNames returns a sorted list of all program names that have masks
func (s *MaskDiscoveryService) GetProgramNames() ([]string, error) {
	masksByProgram, err := s.ScanMasksDirectory()
	if err != nil {
		return nil, err
	}
	
	var programNames []string
	for programName := range masksByProgram {
		programNames = append(programNames, programName)
	}
	
	// Sort for consistent display
	sort.Strings(programNames)
	
	return programNames, nil
}

// CheckDuplicateMaskName checks if a mask name already exists in a program directory
func (s *MaskDiscoveryService) CheckDuplicateMaskName(programName, maskName string) (bool, string, error) {
	if programName == "" {
		return false, "", fmt.Errorf("program name cannot be empty")
	}
	if maskName == "" {
		return false, "", fmt.Errorf("mask name cannot be empty")
	}
	
	// Sanitize inputs
	programName = filepath.Base(programName)
	if programName == "." || programName == ".." {
		return false, "", fmt.Errorf("invalid program name: %s", programName)
	}
	
	// Get program directory path
	programPath := filepath.Join(s.getMasksPath(), programName)
	
	// Check if program directory exists
	if _, err := os.Stat(programPath); os.IsNotExist(err) {
		return false, "", nil // No duplicates if directory doesn't exist
	}
	
	// Check for existing files with the same base name but different extensions
	supportedExtensions := []string{config.PNG, ".jpg", ".jpeg"}
	
	for _, ext := range supportedExtensions {
		filePath := filepath.Join(programPath, maskName+ext)
		if _, err := os.Stat(filePath); err == nil {
			// File exists - return the full filename
			return true, maskName + ext, nil
		}
	}
	
	return false, "", nil
}

// GenerateUniqueMaskName generates a unique mask name by appending a number if duplicates exist
func (s *MaskDiscoveryService) GenerateUniqueMaskName(programName, baseMaskName string) (string, error) {
	if programName == "" {
		return "", fmt.Errorf("program name cannot be empty")
	}
	if baseMaskName == "" {
		return "", fmt.Errorf("base mask name cannot be empty")
	}
	
	// Check if the base name is already unique
	exists, _, err := s.CheckDuplicateMaskName(programName, baseMaskName)
	if err != nil {
		return "", fmt.Errorf("failed to check for duplicates: %w", err)
	}
	
	if !exists {
		return baseMaskName, nil
	}
	
	// Generate unique name by appending numbers
	for i := 1; i <= 999; i++ {
		candidateName := fmt.Sprintf("%s_%d", baseMaskName, i)
		exists, _, err := s.CheckDuplicateMaskName(programName, candidateName)
		if err != nil {
			return "", fmt.Errorf("failed to check for duplicates: %w", err)
		}
		
		if !exists {
			return candidateName, nil
		}
	}
	
	return "", fmt.Errorf("could not generate unique mask name after 999 attempts")
}

// GetMaskConflictInfo returns detailed information about mask name conflicts
func (s *MaskDiscoveryService) GetMaskConflictInfo(programName, maskName string) (*MaskConflictInfo, error) {
	if programName == "" {
		return nil, fmt.Errorf("program name cannot be empty")
	}
	if maskName == "" {
		return nil, fmt.Errorf("mask name cannot be empty")
	}
	
	exists, existingFileName, err := s.CheckDuplicateMaskName(programName, maskName)
	if err != nil {
		return nil, err
	}
	
	conflictInfo := &MaskConflictInfo{
		ProgramName:      programName,
		MaskName:         maskName,
		HasConflict:      exists,
		ExistingFileName: existingFileName,
	}
	
	if exists {
		// Get additional details about the existing file
		existingPath := filepath.Join(s.getMasksPath(), programName, existingFileName)
		if info, err := os.Stat(existingPath); err == nil {
			conflictInfo.ExistingFileSize = info.Size()
			conflictInfo.ExistingFileModTime = info.ModTime()
		}
		
		// Generate suggested alternative name
		if suggestedName, err := s.GenerateUniqueMaskName(programName, maskName); err == nil {
			conflictInfo.SuggestedName = suggestedName
		}
	}
	
	return conflictInfo, nil
}

// MaskConflictInfo contains information about mask name conflicts
type MaskConflictInfo struct {
	ProgramName         string
	MaskName            string
	HasConflict         bool
	ExistingFileName    string
	ExistingFileSize    int64
	ExistingFileModTime time.Time
	SuggestedName       string
}