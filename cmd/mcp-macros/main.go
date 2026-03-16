// MCP server for Squire macros. Exposes tools so an AI can build macros from natural language:
// the user describes what they want, and the AI calls list_macros, create_macro, add_action, etc.
package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"Squire/internal/config"
	"Squire/internal/models/repositories"
	"Squire/internal/models/serialize"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

func main() {
	if err := initConfig(); err != nil {
		log.Fatalf("config init: %v", err)
	}

	server := mcp.NewServer(&mcp.Implementation{Name: "squire-macros", Version: "0.1.0"}, nil)

	mcp.AddTool(server, &mcp.Tool{
		Name:        "list_macros",
		Description: "List all macro names in the Squire config. Use this to see existing macros before creating or editing.",
	}, listMacros)

	mcp.AddTool(server, &mcp.Tool{
		Name:        "get_macro",
		Description: "Get a macro by name. Returns its structure (name, hotkey, global_delay, root action tree) as JSON so you can inspect or extend it.",
	}, getMacro)

	mcp.AddTool(server, &mcp.Tool{
		Name:        "create_macro",
		Description: "Create a new macro. Give it a name; optionally set hotkey (array of strings, e.g. [\"ctrl\",\"shift\",\"m\"]) and global_delay (ms).",
	}, createMacro)

	mcp.AddTool(server, &mcp.Tool{
		Name:        "add_action",
		Description: "Append an action to a macro's root. action_spec must be a JSON object with 'type' and type-specific fields. Types: move (point.x, point.y), click (button: false=left, hold: bool), wait (time: ms), key (key, state), type (text, delayms), loop (count, name, subactions), runmacro (macroname), setvariable (variablename, value), etc. Example: {\"type\":\"move\",\"point\":{\"x\":100,\"y\":100}} then {\"type\":\"click\",\"button\":false} then {\"type\":\"wait\",\"time\":2000}.",
	}, addAction)

	mcp.AddTool(server, &mcp.Tool{
		Name:        "delete_macro",
		Description: "Delete a macro by name.",
	}, deleteMacro)

	if err := server.Run(context.Background(), &mcp.StdioTransport{}); err != nil {
		log.Fatalf("mcp server: %v", err)
	}
}

func initConfig() error {
	configPath := os.Getenv("SQYRE_DB_PATH")
	if configPath == "" {
		configPath = config.GetDbPath()
	}
	dir := filepath.Dir(configPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create config dir: %w", err)
	}
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		body := []byte("macros: {}\nprograms: {}\n")
		if err := os.WriteFile(configPath, body, 0644); err != nil {
			return fmt.Errorf("create config file: %w", err)
		}
	}
	serialize.GetYAMLConfig().SetConfigFile(configPath)
	return serialize.GetYAMLConfig().ReadConfig()
}

type ListMacrosInput struct{}

type ListMacrosOutput struct {
	Macros []string `json:"macros"`
}

func listMacros(ctx context.Context, req *mcp.CallToolRequest, input ListMacrosInput) (*mcp.CallToolResult, ListMacrosOutput, error) {
	repo := repositories.MacroRepo()
	keys := repo.GetAllKeys()
	return nil, ListMacrosOutput{Macros: keys}, nil
}

type GetMacroInput struct {
	Name string `json:"macro_name" jsonschema:"required,description=Name of the macro"`
}

type GetMacroOutput struct {
	Found bool        `json:"found"`
	Macro *macroJSON  `json:"macro,omitempty"`
	Error string      `json:"error,omitempty"`
}

type macroJSON struct {
	Name        string        `json:"name"`
	Hotkey      []string      `json:"hotkey"`
	GlobalDelay int           `json:"global_delay"`
	Root        []interface{} `json:"root_actions"`
}

func getMacro(ctx context.Context, req *mcp.CallToolRequest, input GetMacroInput) (*mcp.CallToolResult, GetMacroOutput, error) {
	repo := repositories.MacroRepo()
	m, err := repo.Get(input.Name)
	if err != nil {
		return nil, GetMacroOutput{Found: false, Error: err.Error()}, nil
	}
	if m == nil || m.Root == nil {
		return nil, GetMacroOutput{Found: true, Macro: &macroJSON{Name: input.Name}}, nil
	}
	rootMap, err := serialize.ActionToMap(m.Root)
	if err != nil {
		return nil, GetMacroOutput{Found: true, Error: err.Error()}, nil
	}
	subs, _ := rootMap["subactions"].([]any)
	out := &macroJSON{
		Name:        m.Name,
		Hotkey:      m.Hotkey,
		GlobalDelay: m.GlobalDelay,
		Root:        nil,
	}
	if len(subs) > 0 {
		out.Root = make([]interface{}, len(subs))
		for i, s := range subs {
			out.Root[i] = s
		}
	}
	return nil, GetMacroOutput{Found: true, Macro: out}, nil
}

type CreateMacroInput struct {
	Name        string   `json:"name" jsonschema:"required,description=Name of the new macro"`
	Hotkey      []string `json:"hotkey,omitempty"`
	GlobalDelay int      `json:"global_delay,omitempty"`
}

type CreateMacroOutput struct {
	OK    bool   `json:"ok"`
	Error string `json:"error,omitempty"`
}

func createMacro(ctx context.Context, req *mcp.CallToolRequest, input CreateMacroInput) (*mcp.CallToolResult, CreateMacroOutput, error) {
	repo := repositories.MacroRepo()
	m := repo.New()
	m.Name = input.Name
	if len(input.Hotkey) > 0 {
		m.Hotkey = input.Hotkey
	}
	if input.GlobalDelay > 0 {
		m.GlobalDelay = input.GlobalDelay
	}
	if err := repo.Set(input.Name, m); err != nil {
		return nil, CreateMacroOutput{OK: false, Error: err.Error()}, nil
	}
	return nil, CreateMacroOutput{OK: true}, nil
}

type AddActionInput struct {
	MacroName  string         `json:"macro_name" jsonschema:"required"`
	ActionSpec map[string]any `json:"action_spec" jsonschema:"required"`
}

type AddActionOutput struct {
	OK    bool   `json:"ok"`
	Error string `json:"error,omitempty"`
}

func addAction(ctx context.Context, req *mcp.CallToolRequest, input AddActionInput) (*mcp.CallToolResult, AddActionOutput, error) {
	if input.ActionSpec == nil {
		return nil, AddActionOutput{OK: false, Error: "action_spec is required"}, nil
	}
	if _, hasType := input.ActionSpec["type"]; !hasType {
		return nil, AddActionOutput{OK: false, Error: "action_spec must include 'type'"}, nil
	}
	repo := repositories.MacroRepo()
	m, err := repo.Get(input.MacroName)
	if err != nil {
		return nil, AddActionOutput{OK: false, Error: err.Error()}, nil
	}
	if m == nil || m.Root == nil {
		return nil, AddActionOutput{OK: false, Error: "macro or root not found"}, nil
	}
	action, err := serialize.ViperSerializer.CreateActionFromMap(input.ActionSpec, m.Root)
	if err != nil {
		return nil, AddActionOutput{OK: false, Error: err.Error()}, nil
	}
	m.Root.AddSubAction(action)
	if err := repo.Set(input.MacroName, m); err != nil {
		return nil, AddActionOutput{OK: false, Error: err.Error()}, nil
	}
	return nil, AddActionOutput{OK: true}, nil
}

type DeleteMacroInput struct {
	Name string `json:"macro_name" jsonschema:"required"`
}

type DeleteMacroOutput struct {
	OK    bool   `json:"ok"`
	Error string `json:"error,omitempty"`
}

func deleteMacro(ctx context.Context, req *mcp.CallToolRequest, input DeleteMacroInput) (*mcp.CallToolResult, DeleteMacroOutput, error) {
	repo := repositories.MacroRepo()
	if err := repo.Delete(input.Name); err != nil {
		return nil, DeleteMacroOutput{OK: false, Error: err.Error()}, nil
	}
	return nil, DeleteMacroOutput{OK: true}, nil
}
