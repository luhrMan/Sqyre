//! JNI calls into `com.sqyre.app.SqyreBridge` and the natives it calls back.
//!
//! Kotlin calls `nativeInit` once from `MainActivity.onCreate`; that caches the VM and
//! a global class ref, because `FindClass` on a native worker thread only sees the
//! system class loader.

use crate::frame::FrameLayout;
use crate::pointer::Gesture;
use crate::{
    frames, insets, parse_app_line, parse_app_list, picks, request_continue, request_stop, status,
    AndroidError, AppIcon, Insets, LaunchableApp,
};
use jni::objects::{GlobalRef, JByteArray, JByteBuffer, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jint};
use jni::{JNIEnv, JavaVM};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

struct Bridge {
    vm: JavaVM,
    class: GlobalRef,
}

static BRIDGE: OnceLock<Bridge> = OnceLock::new();

/// Run `f` with a JNI env attached to this thread and the `SqyreBridge` class.
fn with_bridge<R>(
    f: impl FnOnce(&mut JNIEnv<'static>, &JClass<'static>) -> jni::errors::Result<R>,
) -> Result<R, AndroidError> {
    let bridge = BRIDGE.get().ok_or(AndroidError::NotInitialized)?;
    let mut env = bridge.vm.attach_current_thread_permanently()?;
    let class: &JClass<'static> = bridge.class.as_obj().into();
    let result = f(&mut env, class);
    if env.exception_check()? {
        // Kotlin already returned a status or threw; a pending exception would poison
        // every later JNI call on this thread.
        env.exception_describe()?;
        env.exception_clear()?;
    }
    Ok(result?)
}

/// Dispatch one accessibility gesture and wait for it to finish.
///
/// Must not run on the Android main thread: the gesture callback is delivered there.
pub fn dispatch(gesture: Gesture) -> Result<(), AndroidError> {
    let code = with_bridge(|env, class| match gesture {
        Gesture::Press { x, y, duration_ms } => env
            .call_static_method(
                class,
                "press",
                "(IIJ)I",
                &[
                    JValue::Int(x),
                    JValue::Int(y),
                    JValue::Long(i64::from(duration_ms)),
                ],
            )?
            .i(),
        Gesture::Swipe {
            from,
            to,
            duration_ms,
        } => env
            .call_static_method(
                class,
                "swipe",
                "(IIIIJ)I",
                &[
                    JValue::Int(from.0),
                    JValue::Int(from.1),
                    JValue::Int(to.0),
                    JValue::Int(to.1),
                    JValue::Long(i64::from(duration_ms)),
                ],
            )?
            .i(),
    })?;
    status::check(code)
}

/// Append `text` to the focused editable field in the foreground app.
pub fn append_text(text: &str) -> Result<(), AndroidError> {
    let code = with_bridge(|env, class| {
        let text = env.new_string(text)?;
        env.call_static_method(
            class,
            "appendText",
            "(Ljava/lang/String;)I",
            &[(&text).into()],
        )?
        .i()
    })?;
    status::check(code)
}

pub fn set_clipboard(text: &str) -> Result<(), AndroidError> {
    let code = with_bridge(|env, class| {
        let text = env.new_string(text)?;
        env.call_static_method(
            class,
            "setClipboard",
            "(Ljava/lang/String;)I",
            &[(&text).into()],
        )?
        .i()
    })?;
    status::check(code)
}

/// Launch `package`; a non-empty `label` must equal the app label.
pub fn launch(package: &str, label: &str) -> Result<(), AndroidError> {
    let code = with_bridge(|env, class| {
        let package = env.new_string(package)?;
        let label = env.new_string(label)?;
        env.call_static_method(
            class,
            "launch",
            "(Ljava/lang/String;Ljava/lang/String;)I",
            &[(&package).into(), (&label).into()],
        )?
        .i()
    })?;
    status::check(code)
}

/// Bring Sqyre's activity back in front of the app that covers it.
pub fn show_shell() -> Result<(), AndroidError> {
    let code =
        with_bridge(|env, class| env.call_static_method(class, "showShell", "()I", &[])?.i())?;
    status::check(code)
}

/// Send Sqyre's task to the back so the app used before it comes to the front.
pub fn show_previous() -> Result<(), AndroidError> {
    let code = with_bridge(|env, class| {
        env.call_static_method(class, "showPrevious", "()I", &[])?
            .i()
    })?;
    status::check(code)
}

/// Call a no-arg `SqyreBridge` method that returns a `String`.
fn call_text(method: &str) -> Result<String, AndroidError> {
    with_bridge(|env, class| {
        let obj = JString::from(
            env.call_static_method(class, method, "()Ljava/lang/String;", &[])?
                .l()?,
        );
        let text: String = env.get_string(&obj)?.into();
        // Worker threads stay attached, so local refs are never freed by a returning frame.
        env.delete_local_ref(obj)?;
        Ok(text)
    })
}

/// Apps with a launcher activity, sorted by label.
pub fn launchable_apps() -> Result<Vec<LaunchableApp>, AndroidError> {
    Ok(parse_app_list(&call_text("launchableApps")?))
}

/// `package`'s launcher icon rendered at `side`²; `None` when the app is gone or has none.
pub fn app_icon(package: &str, side: u32) -> Result<Option<AppIcon>, AndroidError> {
    let rgba = with_bridge(|env, class| {
        let package = env.new_string(package)?;
        let array = JByteArray::from(
            env.call_static_method(
                class,
                "appIcon",
                "(Ljava/lang/String;I)[B",
                &[
                    (&package).into(),
                    JValue::Int(i32::try_from(side).unwrap_or(i32::MAX)),
                ],
            )?
            .l()?,
        );
        let rgba = env.convert_byte_array(&array)?;
        // Called once per listed app on an attached worker; free refs to bound the table.
        env.delete_local_ref(array)?;
        env.delete_local_ref(package)?;
        Ok(rgba)
    })?;
    Ok(AppIcon::from_rgba(side, rgba))
}

/// App whose window came to the front last; `None` while the accessibility service is off.
pub fn foreground_app() -> Result<Option<LaunchableApp>, AndroidError> {
    Ok(parse_app_line(&call_text("foregroundApp")?))
}

/// Open the system document picker for pick `id`, limited to `mime_types`.
/// The shell answers through `nativeOnDocumentPicked`.
pub fn pick_document(id: i32, mime_types: &[&str]) -> Result<(), AndroidError> {
    with_bridge(|env, class| {
        let mimes = env.new_object_array(
            i32::try_from(mime_types.len()).unwrap_or(i32::MAX),
            "java/lang/String",
            JObject::null(),
        )?;
        for (i, mime) in (0..).zip(mime_types) {
            let mime = env.new_string(mime)?;
            env.set_object_array_element(&mimes, i, &mime)?;
            env.delete_local_ref(mime)?;
        }
        let result = env
            .call_static_method(
                class,
                "pickDocument",
                "(I[Ljava/lang/String;)V",
                &[JValue::Int(id), (&mimes).into()],
            )
            .and_then(|v| v.v());
        env.delete_local_ref(mimes)?;
        result
    })
}

/// Ask the shell to show the screen-recording consent dialog (no-op while one is pending).
pub fn request_projection() -> Result<(), AndroidError> {
    with_bridge(|env, class| {
        env.call_static_method(class, "requestProjection", "()V", &[])?
            .v()
    })
}

/// Notification permission dialog shown by [`request_notifications`] and not yet answered.
static NOTIFICATIONS_PROMPT_OPEN: AtomicBool = AtomicBool::new(false);

/// Show the notification permission dialog (Android 13+). Returns `false` when there was
/// nothing to ask, so no answer will follow.
pub fn request_notifications() -> Result<bool, AndroidError> {
    NOTIFICATIONS_PROMPT_OPEN.store(true, Ordering::Release);
    let asked = with_bridge(|env, class| {
        env.call_static_method(class, "requestNotifications", "()Z", &[])?
            .z()
    });
    if !matches!(asked, Ok(true)) {
        NOTIFICATIONS_PROMPT_OPEN.store(false, Ordering::Release);
    }
    asked
}

/// True while the dialog from [`request_notifications`] is still up.
pub fn notifications_prompt_open() -> bool {
    NOTIFICATIONS_PROMPT_OPEN.load(Ordering::Acquire)
}

pub fn accessibility_enabled() -> Result<bool, AndroidError> {
    with_bridge(|env, class| {
        env.call_static_method(class, "accessibilityEnabled", "()Z", &[])?
            .z()
    })
}

pub fn open_accessibility_settings() -> Result<(), AndroidError> {
    with_bridge(|env, class| {
        env.call_static_method(class, "openAccessibilitySettings", "()V", &[])?
            .v()
    })
}

/// False when the user blocked Sqyre's notifications, which hides Stop and Continue.
pub fn notifications_enabled() -> Result<bool, AndroidError> {
    with_bridge(|env, class| {
        env.call_static_method(class, "notificationsEnabled", "()Z", &[])?
            .z()
    })
}

pub fn open_notification_settings() -> Result<(), AndroidError> {
    with_bridge(|env, class| {
        env.call_static_method(class, "openNotificationSettings", "()V", &[])?
            .v()
    })
}

/// Real display size in pixels (same space as projection frames and gestures).
pub fn display_size() -> Result<(i32, i32), AndroidError> {
    with_bridge(|env, class| {
        let w = env
            .call_static_method(class, "displayWidth", "()I", &[])?
            .i()?;
        let h = env
            .call_static_method(class, "displayHeight", "()I", &[])?
            .i()?;
        Ok((w, h))
    })
}

/// Natives must not unwind into the JVM.
fn guard(what: &str, f: impl FnOnce() -> Result<(), AndroidError>) {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("sqyre-android: {what}: {e}"),
        Err(_) => eprintln!("sqyre-android: {what}: panicked"),
    }
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeInit(env: JNIEnv, class: JClass) {
    guard("nativeInit", || {
        let vm = env.get_java_vm()?;
        let class = env.new_global_ref(class)?;
        // A recreated activity in the same process sees the same class; keep the first ref.
        let _ = BRIDGE.set(Bridge { vm, class });
        Ok(())
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnFrame(
    env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
    width: jint,
    height: jint,
    row_stride: jint,
    pixel_stride: jint,
) {
    guard("nativeOnFrame", || {
        if !frames().wants_frames() {
            frames().mark_running();
            return Ok(());
        }
        let ptr = env.get_direct_buffer_address(&buffer)?;
        let len = env.get_direct_buffer_capacity(&buffer)?;
        if ptr.is_null() {
            return Err(AndroidError::BadFrame("null direct buffer"));
        }
        // SAFETY: Kotlin keeps the `Image` open until this native returns, so the direct
        // buffer stays mapped and unchanged for `len` bytes for the whole call.
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        frames().publish(
            FrameLayout {
                width,
                height,
                row_stride,
                pixel_stride,
            },
            bytes,
        )
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnProjectionStopped(
    _env: JNIEnv,
    _class: JClass,
) {
    guard("nativeOnProjectionStopped", || {
        frames().mark_stopped();
        Ok(())
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnNotificationsAnswered(
    _env: JNIEnv,
    _class: JClass,
) {
    NOTIFICATIONS_PROMPT_OPEN.store(false, Ordering::Release);
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnShellVisible(
    _env: JNIEnv,
    _class: JClass,
    visible: jboolean,
) {
    guard("nativeOnShellVisible", || {
        frames().set_shell_visible(visible != 0);
        Ok(())
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnStopRequested(
    _env: JNIEnv,
    _class: JClass,
) {
    guard("nativeOnStopRequested", || {
        request_stop();
        Ok(())
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnContinueRequested(
    _env: JNIEnv,
    _class: JClass,
) {
    guard("nativeOnContinueRequested", || {
        request_continue();
        Ok(())
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnInsets(
    _env: JNIEnv,
    _class: JClass,
    left: jint,
    top: jint,
    right: jint,
    bottom: jint,
) {
    guard("nativeOnInsets", || {
        insets().set(Insets::from_px(left, top, right, bottom));
        Ok(())
    });
}

#[no_mangle]
#[allow(
    non_snake_case,
    reason = "JNI symbol names are fixed by the Kotlin class"
)]
pub extern "system" fn Java_com_sqyre_app_SqyreBridge_nativeOnDocumentPicked(
    mut env: JNIEnv,
    _class: JClass,
    id: jint,
    path: JString,
) {
    guard("nativeOnDocumentPicked", || {
        let path: String = env.get_string(&path)?.into();
        picks().complete(id, &path);
        Ok(())
    });
}
