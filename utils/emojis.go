package utils

var Emojis = map[string]string{
	"Move":         "↔️",
	"Click":        "🖱️",
	"Key":          "⌨️",
	"Container":    "🔁",
	"Wait":         "⏳",
	"Image Search": "🔍",
	"OCR":          "🔬",
}

func GetEmoji(key string) string {
	return Emojis[key]
}
