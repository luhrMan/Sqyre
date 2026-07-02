//go:build !linux

package screen

import "github.com/vcaesar/screenshot"

func numDisplaysImpl() int {
	return screenshot.NumActiveDisplays()
}
