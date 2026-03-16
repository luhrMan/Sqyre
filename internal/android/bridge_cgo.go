//go:build android && cgo

package android

/*
#include <stdint.h>
#include <stdlib.h>

void open_accessibility_settings(uintptr_t env, uintptr_t ctx);
void request_notification_permission(uintptr_t env, uintptr_t ctx);
int is_accessibility_enabled(uintptr_t env, uintptr_t ctx);
void open_battery_optimization_settings(uintptr_t env, uintptr_t ctx);

int perform_tap(uintptr_t env, uintptr_t ctx, int x, int y);
void type_text(uintptr_t env, uintptr_t ctx, const char* text, int delay_ms);
int key_event(uintptr_t env, uintptr_t ctx, const char* key, int down);
char* get_pixel_color(uintptr_t env, uintptr_t ctx, int x, int y);
char* get_window_names(uintptr_t env, uintptr_t ctx);
int focus_window(uintptr_t env, uintptr_t ctx, const char* target);
*/
import "C"

import (
	"strings"
	"unsafe"
)

func init() {
	openAccessibilitySettingsFn = func(env, ctx uintptr) {
		C.open_accessibility_settings(C.uintptr_t(env), C.uintptr_t(ctx))
	}
	requestNotificationPermissionFn = func(env, ctx uintptr) {
		C.request_notification_permission(C.uintptr_t(env), C.uintptr_t(ctx))
	}
	isAccessibilityEnabledFn = func(env, ctx uintptr) bool {
		return C.is_accessibility_enabled(C.uintptr_t(env), C.uintptr_t(ctx)) != 0
	}
	openBatteryOptimizationSettingsFn = func(env, ctx uintptr) {
		C.open_battery_optimization_settings(C.uintptr_t(env), C.uintptr_t(ctx))
	}

	performTapNativeFn = func(env, ctx uintptr, x, y int) error {
		if C.perform_tap(C.uintptr_t(env), C.uintptr_t(ctx), C.int(x), C.int(y)) == 0 {
			return ErrAccessibilityRequired
		}
		return nil
	}
	keyEventNativeFn = func(env, ctx uintptr, key string, down bool) error {
		cKey := C.CString(key)
		defer C.free(unsafe.Pointer(cKey))
		downInt := 0
		if down {
			downInt = 1
		}
		if C.key_event(C.uintptr_t(env), C.uintptr_t(ctx), cKey, C.int(downInt)) == 0 {
			return ErrAccessibilityRequired
		}
		return nil
	}
	typeTextNativeFn = func(env, ctx uintptr, text string, delayMs int) error {
		cText := C.CString(text)
		defer C.free(unsafe.Pointer(cText))
		C.type_text(C.uintptr_t(env), C.uintptr_t(ctx), cText, C.int(delayMs))
		return nil
	}
	getPixelColorNativeFn = func(env, ctx uintptr, x, y int) (string, error) {
		cStr := C.get_pixel_color(C.uintptr_t(env), C.uintptr_t(ctx), C.int(x), C.int(y))
		if cStr == nil {
			return "", ErrAccessibilityRequired
		}
		defer C.free(unsafe.Pointer(cStr))
		return C.GoString(cStr), nil
	}
	windowNamesNativeFn = func(env, ctx uintptr) ([]string, error) {
		cStr := C.get_window_names(C.uintptr_t(env), C.uintptr_t(ctx))
		if cStr == nil {
			return nil, nil
		}
		defer C.free(unsafe.Pointer(cStr))
		s := C.GoString(cStr)
		if s == "" {
			return nil, nil
		}
		return strings.Split(s, "\n"), nil
	}
	focusWindowNativeFn = func(env, ctx uintptr, windowTarget string) error {
		cTarget := C.CString(windowTarget)
		defer C.free(unsafe.Pointer(cTarget))
		if C.focus_window(C.uintptr_t(env), C.uintptr_t(ctx), cTarget) == 0 {
			return ErrAccessibilityRequired
		}
		return nil
	}
}
