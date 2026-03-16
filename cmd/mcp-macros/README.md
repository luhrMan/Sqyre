# Squire Macros MCP Server

MCP server that exposes Squire macros as tools so an AI (e.g. in Cursor) can create and edit macros from **natural language**. The user describes what they want; the AI turns that into calls to `list_macros`, `create_macro`, `add_action`, etc.

## Build and run

```bash
# From repo root
go build -o mcp-macros ./cmd/mcp-macros
./mcp-macros
```

The server uses stdio transport (stdin/stdout). Config path defaults to `~/.sqyre/db.yaml`; override with `SQYRE_DB_PATH`.

## Cursor setup

Add to Cursor MCP settings (e.g. **Cursor Settings → MCP**), using the full path to the binary:

```json
{
  "mcpServers": {
    "squire-macros": {
      "command": "/absolute/path/to/mcp-macros",
      "env": {}
    }
  }
}
```

Or run via Go:

```json
{
  "mcpServers": {
    "squire-macros": {
      "command": "go",
      "args": ["run", "./cmd/mcp-macros"],
      "cwd": "/path/to/Squire"
    }
  }
}
```

## Tools

- **list_macros** – List all macro names.
- **get_macro** – Get a macro by name (structure as JSON).
- **create_macro** – Create a new macro (name; optional hotkey, global_delay).
- **add_action** – Append an action to a macro. `action_spec` is a JSON object with `type` and type-specific fields (see design doc).
- **delete_macro** – Delete a macro by name.

See [docs/MCP-MACROS-DESIGN.md](../../docs/MCP-MACROS-DESIGN.md) for the full design and action schema.
