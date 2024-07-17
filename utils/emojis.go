package utils

var Emojis = map[string]string{
	"Move":         "↔️",
	"Click":        "🖱️",
	"Key":          "⌨️",
	"Wait":         "⏳",
	"Image Search": "🔍",
	"OCR":          "🔬",
	"Loop":         "🔁",
	"Conditional":  "❓",
}

func GetEmoji(key string) string {
	return Emojis[key]
}
