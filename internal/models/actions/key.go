package actions

type Key struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Key         string
	State       bool
}

func NewKey(key string, state bool) *Key {
	return &Key{
		BaseAction: newBaseAction("key"),
		Key:        key,
		State:      state,
	}
}

func (a *Key) String() string           { return stringifyParams(a.parameters()) }
func (a *Key) Parameters() []ActionParam { return a.parameters() }

func (a *Key) parameters() []ActionParam {
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Key", a.Key),
		newParam("State", UpOrDown(a.State)),
	}
}

func UpOrDown(b bool) string {
	if b {
		return "down"
	}
	return "up"
}
