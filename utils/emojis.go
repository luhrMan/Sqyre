package utils

var Emojis = map[string]string{
	"Move":     "↔️",
	"Click":    "🖱️",
	"Key":      "⌨️",
	"Sequence": "🔢",
	"Wait":     "⏳",
}

func GetEmoji(key string) string {
	return Emojis[key]
}
