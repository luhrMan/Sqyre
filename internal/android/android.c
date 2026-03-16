#include <jni.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define FLAG_ACTIVITY_NEW_TASK 0x10000000
#define SQYRE_BRIDGE_CLASS "com/sqyre/app/SqyreBridge"

static void open_accessibility_settings_impl(JNIEnv *env, jobject ctx) {
	jclass contextClass = (*env)->GetObjectClass(env, ctx);
	if (!contextClass) return;
	jmethodID startActivityId = (*env)->GetMethodID(env, contextClass, "startActivity", "(Landroid/content/Intent;)V");
	if (!startActivityId) return;

	jclass intentClass = (*env)->FindClass(env, "android/content/Intent");
	if (!intentClass) return;
	jmethodID intentInit = (*env)->GetMethodID(env, intentClass, "<init>", "(Ljava/lang/String;)V");
	if (!intentInit) return;

	jstring action = (*env)->NewStringUTF(env, "android.settings.ACCESSIBILITY_SETTINGS");
	if (!action) return;
	jobject intent = (*env)->NewObject(env, intentClass, intentInit, action);
	if (!intent) return;

	jmethodID setFlagsId = (*env)->GetMethodID(env, intentClass, "setFlags", "(I)Landroid/content/Intent;");
	if (setFlagsId) {
		(*env)->CallObjectMethod(env, intent, setFlagsId, (jint)FLAG_ACTIVITY_NEW_TASK);
	}
	(*env)->CallVoidMethod(env, ctx, startActivityId, intent);
}

void open_accessibility_settings(uintptr_t env, uintptr_t ctx) {
	open_accessibility_settings_impl((JNIEnv *)env, (jobject)ctx);
}

static void request_notification_permission_impl(JNIEnv *env, jobject ctx) {
	/* Android 13+ POST_NOTIFICATIONS - would need Activity.requestPermissions */
	/* For now we open app notification settings so user can enable */
	jclass contextClass = (*env)->GetObjectClass(env, ctx);
	if (!contextClass) return;
	jmethodID startActivityId = (*env)->GetMethodID(env, contextClass, "startActivity", "(Landroid/content/Intent;)V");
	if (!startActivityId) return;

	jclass intentClass = (*env)->FindClass(env, "android/content/Intent");
	if (!intentClass) return;
	jmethodID intentInit = (*env)->GetMethodID(env, intentClass, "<init>", "(Ljava/lang/String;)V");
	if (!intentInit) return;

	jstring action = (*env)->NewStringUTF(env, "android.settings.APP_NOTIFICATION_SETTINGS");
	if (!action) return;
	jobject intent = (*env)->NewObject(env, intentClass, intentInit, action);
	if (!intent) return;

	jmethodID setFlagsId = (*env)->GetMethodID(env, intentClass, "setFlags", "(I)Landroid/content/Intent;");
	if (setFlagsId) {
		(*env)->CallObjectMethod(env, intent, setFlagsId, (jint)FLAG_ACTIVITY_NEW_TASK);
	}
	(*env)->CallVoidMethod(env, ctx, startActivityId, intent);
}

void request_notification_permission(uintptr_t env, uintptr_t ctx) {
	request_notification_permission_impl((JNIEnv *)env, (jobject)ctx);
}

static int is_accessibility_enabled_impl(JNIEnv *env, jobject ctx) {
	jclass bridgeClass = (*env)->FindClass(env, SQYRE_BRIDGE_CLASS);
	if (!bridgeClass) return 0;
	jmethodID isEnabledId = (*env)->GetStaticMethodID(env, bridgeClass, "isServiceEnabled", "()Z");
	if (!isEnabledId) return 0;
	jboolean ok = (*env)->CallStaticBooleanMethod(env, bridgeClass, isEnabledId);
	return (int)ok;
}

int is_accessibility_enabled(uintptr_t env, uintptr_t ctx) {
	return is_accessibility_enabled_impl((JNIEnv *)env, (jobject)ctx);
}

static void open_battery_optimization_settings_impl(JNIEnv *env, jobject ctx) {
	jclass contextClass = (*env)->GetObjectClass(env, ctx);
	if (!contextClass) return;
	jmethodID startActivityId = (*env)->GetMethodID(env, contextClass, "startActivity", "(Landroid/content/Intent;)V");
	if (!startActivityId) return;

	jclass intentClass = (*env)->FindClass(env, "android/content/Intent");
	if (!intentClass) return;
	jmethodID intentInit = (*env)->GetMethodID(env, intentClass, "<init>", "(Ljava/lang/String;)V");
	if (!intentInit) return;

	jstring action = (*env)->NewStringUTF(env, "android.settings.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS");
	if (!action) return;
	jobject intent = (*env)->NewObject(env, intentClass, intentInit, action);
	if (!intent) return;

	jmethodID setDataId = (*env)->GetMethodID(env, intentClass, "setData", "(Landroid/net/Uri;)Landroid/content/Intent;");
	if (setDataId) {
		jclass uriClass = (*env)->FindClass(env, "android/net/Uri");
		if (uriClass) {
			jmethodID parseId = (*env)->GetStaticMethodID(env, uriClass, "parse", "(Ljava/lang/String;)Landroid/net/Uri;");
			if (parseId) {
				jstring packageUri = (*env)->NewStringUTF(env, "package:com.sqyre.app");
				if (packageUri) {
					jobject uri = (*env)->CallStaticObjectMethod(env, uriClass, parseId, packageUri);
					if (uri) {
						(*env)->CallObjectMethod(env, intent, setDataId, uri);
					}
				}
			}
		}
	}
	jmethodID setFlagsId = (*env)->GetMethodID(env, intentClass, "setFlags", "(I)Landroid/content/Intent;");
	if (setFlagsId) {
		(*env)->CallObjectMethod(env, intent, setFlagsId, (jint)FLAG_ACTIVITY_NEW_TASK);
	}
	(*env)->CallVoidMethod(env, ctx, startActivityId, intent);
}

void open_battery_optimization_settings(uintptr_t env, uintptr_t ctx) {
	open_battery_optimization_settings_impl((JNIEnv *)env, (jobject)ctx);
}

/* --- SqyreBridge JNI: tap, type, key, getPixelColor, getWindowNames, focusWindow --- */

static jclass get_bridge_class(JNIEnv *env) {
	return (*env)->FindClass(env, SQYRE_BRIDGE_CLASS);
}

int perform_tap(uintptr_t env, uintptr_t ctx, int x, int y) {
	JNIEnv *e = (JNIEnv *)env;
	jclass c = get_bridge_class(e);
	if (!c) return 0;
	jmethodID mid = (*e)->GetStaticMethodID(e, c, "performTap", "(II)Z");
	if (!mid) return 0;
	jboolean ok = (*e)->CallStaticBooleanMethod(e, c, mid, (jint)x, (jint)y);
	return (int)ok;
}

void type_text(uintptr_t env, uintptr_t ctx, const char *text, int delay_ms) {
	JNIEnv *e = (JNIEnv *)env;
	jclass c = get_bridge_class(e);
	if (!c) return;
	jmethodID mid = (*e)->GetStaticMethodID(e, c, "typeText", "(Ljava/lang/String;I)V");
	if (!mid) return;
	jstring jtext = (*e)->NewStringUTF(e, text ? text : "");
	if (jtext) {
		(*e)->CallStaticVoidMethod(e, c, mid, jtext, (jint)delay_ms);
	}
}

int key_event(uintptr_t env, uintptr_t ctx, const char *key, int down) {
	JNIEnv *e = (JNIEnv *)env;
	jclass c = get_bridge_class(e);
	if (!c) return 0;
	jmethodID mid = (*e)->GetStaticMethodID(e, c, "keyEvent", "(Ljava/lang/String;Z)Z");
	if (!mid) return 0;
	jstring jkey = (*e)->NewStringUTF(e, key ? key : "");
	if (!jkey) return 0;
	jboolean ok = (*e)->CallStaticBooleanMethod(e, c, mid, jkey, (jboolean)down);
	return (int)ok;
}

/* Caller must free the returned string. */
char *get_pixel_color(uintptr_t env, uintptr_t ctx, int x, int y) {
	JNIEnv *e = (JNIEnv *)env;
	jclass c = get_bridge_class(e);
	if (!c) return NULL;
	jmethodID mid = (*e)->GetStaticMethodID(e, c, "getPixelColor", "(II)Ljava/lang/String;");
	if (!mid) return NULL;
	jstring jstr = (jstring)(*e)->CallStaticObjectMethod(e, c, mid, (jint)x, (jint)y);
	if (!jstr) return NULL;
	const char *utf = (*e)->GetStringUTFChars(e, jstr, NULL);
	if (!utf) return NULL;
	char *out = strdup(utf);
	(*e)->ReleaseStringUTFChars(e, jstr, utf);
	return out;
}

/* Returns newline-separated window names; caller must free. */
char *get_window_names(uintptr_t env, uintptr_t ctx) {
	JNIEnv *e = (JNIEnv *)env;
	jclass c = get_bridge_class(e);
	if (!c) return strdup("");
	jmethodID mid = (*e)->GetStaticMethodID(e, c, "getWindowNames", "()[Ljava/lang/String;");
	if (!mid) return strdup("");
	jobjectArray arr = (jobjectArray)(*e)->CallStaticObjectMethod(e, c, mid);
	if (!arr) return strdup("");
	jsize len = (*e)->GetArrayLength(e, arr);
	size_t total = 1;
	for (jsize i = 0; i < len; i++) {
		jstring s = (jstring)(*e)->GetObjectArrayElement(e, arr, i);
		if (s) {
			const char *utf = (*e)->GetStringUTFChars(e, s, NULL);
			if (utf) {
				total += strlen(utf) + 1;
				(*e)->ReleaseStringUTFChars(e, s, utf);
			}
			(*e)->DeleteLocalRef(e, s);
		}
	}
	char *buf = (char *)malloc(total);
	if (!buf) return strdup("");
	buf[0] = '\0';
	for (jsize i = 0; i < len; i++) {
		jstring s = (jstring)(*e)->GetObjectArrayElement(e, arr, i);
		if (s) {
			const char *utf = (*e)->GetStringUTFChars(e, s, NULL);
			if (utf) {
				if (buf[0]) strcat(buf, "\n");
				strcat(buf, utf);
				(*e)->ReleaseStringUTFChars(e, s, utf);
			}
			(*e)->DeleteLocalRef(e, s);
		}
	}
	return buf;
}

int focus_window(uintptr_t env, uintptr_t ctx, const char *target) {
	JNIEnv *e = (JNIEnv *)env;
	jclass c = get_bridge_class(e);
	if (!c) return 0;
	jmethodID mid = (*e)->GetStaticMethodID(e, c, "focusWindow", "(Ljava/lang/String;)Z");
	if (!mid) return 0;
	jstring jtarget = (*e)->NewStringUTF(e, target ? target : "");
	if (!jtarget) return 0;
	jboolean ok = (*e)->CallStaticBooleanMethod(e, c, mid, jtarget);
	return (int)ok;
}
