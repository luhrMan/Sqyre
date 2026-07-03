package config

// BoolPreference reads a user preference when the UI has wired fyne prefs at startup.
var BoolPreference func(key string, fallback bool) bool

// StringPreference reads a user preference when the UI has wired fyne prefs at startup.
var StringPreference func(key string) string

func PrefBool(key string, fallback bool) bool {
	if BoolPreference != nil {
		return BoolPreference(key, fallback)
	}
	return fallback
}

func PrefString(key string) string {
	if StringPreference != nil {
		return StringPreference(key)
	}
	return ""
}
