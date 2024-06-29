package structs

type Action struct {
	ActionType string                 // e.g., "MoveToScreen", "Search", "Click"
	Parameters map[string]interface{} // Any parameters needed for the action
}
