# MCP for Natural-Language Macro Building

## Overview

An MCP (Model Context Protocol) server allows Cursor (or any MCP client) to create and edit Squire macros via tools. **Natural language** is handled by the AI in the chat: the user describes what they want (“create a macro that clicks at 100,100 then waits 2 seconds”), and the AI turns that into a sequence of MCP tool calls with structured arguments.

No extra LLM is required inside the MCP server; the assistant interprets the user’s intent and calls the macro tools with the right parameters.

## Architecture

```
User: "Add a macro that moves to 200,300, left-clicks, then waits 1 second"
    → AI interprets intent
    → MCP tool calls: create_macro("My Macro"); add_action("My Macro", {type:"move", point:{x:200,y:300}});
                     add_action("My Macro", {type:"click", button:false}); add_action("My Macro", {type:"wait", time:1000})
    → MCP server updates ~/.sqyre/db.yaml and returns success
```

- **MCP server**: Reads/writes the same Squire config (`~/.sqyre/db.yaml` by default). Exposes tools: `list_macros`, `get_macro`, `create_macro`, `add_action`, `delete_macro`.
- **AI (Cursor)**: Maps natural language to those tool calls and parameters. Knows the action schema (e.g. `move` needs `point.x`, `point.y`; `click` needs `button`; `wait` needs `time` in ms).

## MCP Tools

| Tool | Purpose |
|------|--------|
| `list_macros` | Return all macro names. |
| `get_macro` | Return one macro’s structure (name, hotkey, global delay, root action tree) as JSON so the AI can inspect or extend it. |
| `create_macro` | Create a new macro by name; optional hotkey and global delay. |
| `add_action` | Append an action to a macro’s root. `action_spec` is a JSON object with `type` and type-specific fields (see below). |
| `delete_macro` | Remove a macro by name. |

## Action schema (for `add_action`)

`action_spec` must include `type` and any required fields. Same structure as the app’s internal action map (see `internal/models/serialize/viper.go` and `action_to_map.go`).

| type | Required / important fields | Notes |
|------|----------------------------|--------|
| `move` | `point`: `{ "x": number, "y": number }` | Optional `name`. |
| `click` | `button`: bool (false=left, true=right), `hold`: bool | |
| `wait` | `time`: number (milliseconds) | |
| `key` | `key`: string, `state`: bool (true=key down, false=key up) | |
| `type` | `text`: string, optional `delayms`: number | Types text. |
| `loop` | `count`: number, `name`: string | Sub-actions in `subactions` array. |
| `runmacro` | `macroname`: string | Runs another macro. |
| `setvariable` | `variablename`: string, `value`: string | |
| `imagesearch` | `name`, `targets` ([]string), `searcharea`, `rowsplit`, `colsplit`, `tolerance`, `blur` | More complex; prefer UI for full setup. |
| `ocr` | `name`, `target`, `searcharea` | Same. |

Example for “move to (100,100), then left-click, then wait 2s”:

1. `add_action("My Macro", { "type": "move", "point": { "x": 100, "y": 100 } })`
2. `add_action("My Macro", { "type": "click", "button": false, "hold": false })`
3. `add_action("My Macro", { "type": "wait", "time": 2000 })`

## Config path

The server uses the same config as the Squire app. Default: `~/.sqyre/db.yaml`. Override with env `SQYRE_DB_PATH` if needed.

## Cursor configuration

Add the MCP server to Cursor (e.g. in `.cursor/mcp.json` or Cursor settings):

```json
{
  "mcpServers": {
    "squire-macros": {
      "command": "/path/to/squire/cmd/mcp-macros/mcp-macros",
      "env": {}
    }
  }
}
```

Or use `go run`:

```json
"command": "go",
"args": ["run", "./cmd/mcp-macros", "-config", "/path/to/db.yaml"]
```

## Optional: server-side NL

A possible extension is a single tool `create_macro_from_description(description: string)` that calls an LLM (with an API key) to turn free text into an action list, then creates the macro. That would require the MCP server to have network access and a configured API key; the design above avoids that and keeps all NL in the chat AI.
