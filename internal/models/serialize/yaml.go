package serialize

import (
	"fmt"
	"os"
	"sync"

	"gopkg.in/yaml.v3"
)

// YAMLConfig manages case-sensitive YAML configuration
type YAMLConfig struct {
	mu         sync.RWMutex
	configFile string
	data       map[string]any
}

var (
	yamlConfig     *YAMLConfig
	yamlConfigOnce sync.Once
)

// GetYAMLConfig returns the singleton YAML config instance
func GetYAMLConfig() *YAMLConfig {
	yamlConfigOnce.Do(func() {
		yamlConfig = &YAMLConfig{
			data: make(map[string]any),
		}
	})
	return yamlConfig
}

// SetConfigFile sets the path to the YAML config file
func (c *YAMLConfig) SetConfigFile(path string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.configFile = path
}

// GetConfigFile returns the current config file path
func (c *YAMLConfig) GetConfigFile() string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.configFile
}

// ReadConfig reads the YAML config file from disk
func (c *YAMLConfig) ReadConfig() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.configFile == "" {
		return fmt.Errorf("config file path not set")
	}

	data, err := os.ReadFile(c.configFile)
	if err != nil {
		return fmt.Errorf("failed to read config file: %w", err)
	}

	c.data = make(map[string]any)
	if err := yaml.Unmarshal(data, &c.data); err != nil {
		return fmt.Errorf("failed to unmarshal YAML: %w", err)
	}

	return nil
}

// WriteConfig writes the current config to disk
func (c *YAMLConfig) WriteConfig() error {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if c.configFile == "" {
		return fmt.Errorf("config file path not set")
	}

	data, err := yaml.Marshal(c.data)
	if err != nil {
		return fmt.Errorf("failed to marshal YAML: %w", err)
	}

	if err := os.WriteFile(c.configFile, data, 0644); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	return nil
}

// Get retrieves a value by key (case-sensitive)
func (c *YAMLConfig) Get(key string) any {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.data[key]
}

// GetStringMap retrieves a map[string]any by key
func (c *YAMLConfig) GetStringMap(key string) map[string]any {
	c.mu.RLock()
	defer c.mu.RUnlock()

	val, ok := c.data[key]
	if !ok {
		return make(map[string]any)
	}

	if m, ok := val.(map[string]any); ok {
		return m
	}

	return make(map[string]any)
}

// Set sets a value by key (case-sensitive)
func (c *YAMLConfig) Set(key string, value any) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.data[key] = value
}

// Clear removes all data
func (c *YAMLConfig) Clear() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.data = make(map[string]any)
}
