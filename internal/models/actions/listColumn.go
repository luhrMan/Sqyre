package actions

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"Sqyre/internal/config"
)

// ListColumn is one line-based source bound to an output variable.
type ListColumn struct {
	Source         string `yaml:"source" mapstructure:"source"`
	OutputVar      string `yaml:"outputvar" mapstructure:"outputvar"`
	IsFile         bool   `yaml:"isfile,omitempty" mapstructure:"isfile"`
	SkipBlankLines bool   `yaml:"skipblanklines,omitempty" mapstructure:"skipblanklines"`

	currentLine     int      `yaml:"-" gob:"-" mapstructure:"-"`
	lines           []string `yaml:"-" gob:"-" mapstructure:"-"`
	loadedSourceKey string   `yaml:"-" gob:"-" mapstructure:"-"`
}

func (c *ListColumn) sourceKey() string {
	return fmt.Sprintf("%t|%t|%s", c.IsFile, c.SkipBlankLines, c.Source)
}

func (c *ListColumn) ensureLoaded() error {
	key := c.sourceKey()
	if c.loadedSourceKey == key {
		return nil
	}
	if err := c.loadLines(); err != nil {
		c.lines = nil
		c.loadedSourceKey = ""
		return err
	}
	c.loadedSourceKey = key
	c.currentLine = 0
	return nil
}

func (c *ListColumn) LineCount() (int, error) {
	if err := c.ensureLoaded(); err != nil {
		return 0, err
	}
	return len(c.lines), nil
}

func (c *ListColumn) GetCurrentLine() (string, error) {
	if err := c.ensureLoaded(); err != nil {
		return "", err
	}
	if c.currentLine >= len(c.lines) {
		return "", fmt.Errorf("line index %d out of range (total: %d)", c.currentLine, len(c.lines))
	}
	return c.lines[c.currentLine], nil
}

func (c *ListColumn) CurrentLineIndex() int {
	return c.currentLine
}

func (c *ListColumn) SetLineIndex(index int) {
	c.currentLine = index
}

func (c *ListColumn) NextLine() {
	c.currentLine++
	if c.currentLine >= len(c.lines) {
		c.currentLine = 0
	}
}

func (c *ListColumn) Reset() {
	c.currentLine = 0
	c.lines = nil
	c.loadedSourceKey = ""
}

func (c *ListColumn) loadLines() error {
	if c.IsFile {
		path := c.Source
		if !filepath.IsAbs(path) {
			path = filepath.Join(config.GetVariablesPath(), path)
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return fmt.Errorf("failed to read file %s: %w", path, err)
		}
		c.lines = strings.Split(string(data), "\n")
	} else {
		c.lines = strings.Split(c.Source, "\n")
	}
	for len(c.lines) > 0 && strings.TrimSpace(c.lines[len(c.lines)-1]) == "" {
		c.lines = c.lines[:len(c.lines)-1]
	}
	if c.SkipBlankLines {
		filtered := c.lines[:0]
		for _, line := range c.lines {
			if strings.TrimSpace(line) != "" {
				filtered = append(filtered, line)
			}
		}
		c.lines = filtered
	}
	return nil
}
