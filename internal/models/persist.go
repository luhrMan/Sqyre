package models

// PersistProgram persists the program aggregate after nested repositories (items,
// points, masks, etc.) mutate in-memory state. Set by appdata.Register from the
// active program store; when nil, repositories fall back to ProgramRepo().Set.
var PersistProgram func(*Program) error
