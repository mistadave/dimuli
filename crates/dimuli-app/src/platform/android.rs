//! Android platform: JNI bridge to the Kotlin activity.
// The only module allowed to use `unsafe` (docs/CODING_GUIDELINES.md §3): JNI needs it.
#![allow(unsafe_code)]

use std::rc::Rc;

use jni::errors::LogErrorAndDefault;
use jni::objects::{JObject, JObjectArray, JString};
use jni::refs::Global;
use jni::{Env, JValue, JavaVM, jni_sig, jni_str, native_method};
use slint::android::AndroidApp;

use super::native_events::{images_picked_event, pick_failed_event, saved_event, shared_event};
use super::{Platform, PlatformEvent, post_event};
use crate::texts;

/// Called by android-activity on its own `android_main` thread (may run more than once per process).
#[unsafe(no_mangle)] // SAFETY: unique symbol looked up by android-activity's NativeActivity glue.
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("dimuli")
            .with_max_level(log::LevelFilter::Info),
    );
    log::info!("dimuli started");
    if let Err(error) = slint::android::init(app.clone()) {
        log::error!("cannot init slint: {error}");
        return;
    }
    let mut platform = AndroidPlatform {
        app,
        collage_lost: false,
    };
    if let Err(error) = platform.register_natives() {
        log::error!("cannot register native methods: {error}");
    }
    platform.collage_lost = match platform.with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("collageLostOnRestart"),
            jni_sig!(() -> jboolean),
            &[],
        )?
        .z()
    }) {
        Ok(value) => value,
        Err(error) => {
            log::error!("cannot read restart status: {error}");
            false
        }
    };
    // Strictly after `slint::android::init`: Slint registers its own back callback on API >= 34
    // and the last registered callback wins.
    platform.call_void(jni_str!("installBackCallback"));
    if let Err(error) = crate::run(Rc::new(platform)) {
        log::error!("UI failed: {error}");
    }
}

/// Platform services on Android. Holds the `AndroidApp` itself, never in a static: `android_main`
/// can run again in the same process after `finish()`.
pub(crate) struct AndroidPlatform {
    app: AndroidApp,
    collage_lost: bool,
}

impl AndroidPlatform {
    fn with_activity<R>(
        &self,
        f: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<R>,
    ) -> jni::errors::Result<R> {
        let vm = JavaVM::singleton()?;
        let raw = self.app.activity_as_ptr() as jni::sys::jobject;
        vm.attach_current_thread(|env| {
            // SAFETY: `raw` is android-activity's unowned global ref to the Activity; it stays valid
            // while `self.app` lives, and the borrowing cast never deletes it.
            let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw)? };
            // The android_main thread stays attached for the whole app life, so local references made
            // by `f` (arrays, classes) must be freed per call or they pile up with every save/share.
            env.with_local_frame(16, |env| f(env, activity.as_ref()))
        })
    }

    fn register_natives(&self) -> jni::errors::Result<()> {
        self.with_activity(|env, activity| {
            let class = env.get_object_class(activity)?;
            // SAFETY: every NativeMethod comes from `native_method!`, which checks the Rust signature
            // against the Java signature at compile time.
            unsafe { env.register_native_methods(&class, &NATIVES) }
        })
    }

    /// Calls the Kotlin method `name(jpeg: ByteArray)`; it returns at once and reports the result
    /// later through `nativeSaved` / `nativeShared`.
    fn hand_over(&self, method: &jni::strings::JNIStr, jpeg: &[u8]) -> jni::errors::Result<()> {
        self.with_activity(|env, activity| {
            let bytes = env.byte_array_from_slice(jpeg)?;
            env.call_method(
                activity,
                method,
                jni_sig!((jpeg: jbyte[]) -> void),
                &[JValue::Object(&bytes)],
            )?;
            Ok(())
        })
    }

    /// Calls a `() -> void` method on the Activity; failures are only logged.
    fn call_void(&self, name: &jni::strings::JNIStr) {
        let result = self.with_activity(|env, activity| {
            env.call_method(activity, name, jni_sig!(() -> void), &[])?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("JNI call failed: {error}");
        }
    }
}

impl Platform for AndroidPlatform {
    fn pick_images(&self, max: usize) {
        let max = i32::try_from(max).unwrap_or(i32::MAX);
        let result = self.with_activity(|env, activity| {
            env.call_method(
                activity,
                jni_str!("pickImages"),
                jni_sig!((max: jint) -> void),
                &[JValue::Int(max)],
            )?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("cannot open the photo picker: {error}");
            post_event(pick_failed_event());
        }
    }

    fn save_to_gallery(&self, jpeg: Vec<u8>) {
        if let Err(error) = self.hand_over(jni_str!("saveToGallery"), &jpeg) {
            log::error!("cannot hand the collage to MediaStore: {error}");
            post_event(PlatformEvent::Saved(Err(texts::SAVE_FAILED.to_owned())));
        }
    }

    fn share_supported(&self) -> bool {
        true
    }

    fn restored_after_close(&self) -> bool {
        self.collage_lost
    }

    fn set_collage_open(&self, open: bool) {
        let result = self.with_activity(|env, activity| {
            env.call_method(
                activity,
                jni_str!("setCollageOpen"),
                jni_sig!((open: jboolean) -> void),
                &[JValue::Bool(open)],
            )?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("cannot set collage state: {error}");
        }
    }

    fn share(&self, jpeg: Vec<u8>) {
        if let Err(error) = self.hand_over(jni_str!("share"), &jpeg) {
            log::error!("cannot open the share sheet: {error}");
            post_event(PlatformEvent::Shared(Err(texts::SHARE_FAILED.to_owned())));
        }
    }

    fn close_app(&self) {
        self.call_void(jni_str!("finish"));
    }
}

// jni 0.22.4's macro expansion calls AtomicBool::fetch_update (deprecated since Rust 1.99).
#[allow(deprecated)]
const NATIVES: [jni::NativeMethod<'static>; 6] = [
    native_method! {
        fn native_back_pressed(),
        fn = native_back_pressed,
        error_policy = LogErrorAndDefault
    },
    native_method! {
        fn native_images_picked(paths: java.lang.String[], failed: jint),
        fn = native_images_picked,
        error_policy = LogErrorAndDefault
    },
    native_method! {
        fn native_pick_copying(),
        fn = native_pick_copying,
        error_policy = LogErrorAndDefault
    },
    native_method! {
        fn native_pick_failed(),
        fn = native_pick_failed,
        error_policy = LogErrorAndDefault
    },
    native_method! {
        fn native_saved(ok: jboolean),
        fn = native_saved,
        error_policy = LogErrorAndDefault
    },
    native_method! {
        fn native_shared(ok: jboolean),
        fn = native_shared,
        error_policy = LogErrorAndDefault
    },
];

/// Back gesture seen by Kotlin; JNI natives never touch Slint objects, they only post an event.
fn native_back_pressed<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> jni::errors::Result<()> {
    post_event(PlatformEvent::BackPressed);
    Ok(())
}

/// Cache copies of the picked images (empty array = cancelled), reported by Kotlin.
fn native_images_picked<'local>(
    env: &mut Env<'local>,
    _this: JObject<'local>,
    paths: JObjectArray<'local, JString<'local>>,
    failed: i32,
) -> jni::errors::Result<()> {
    let count = paths.len(env)?;
    let mut list = Vec::with_capacity(count);
    for index in 0..count {
        list.push(paths.get_element(env, index)?.try_to_string(env)?);
    }
    log::info!("picked images: {}, failed copies: {failed}", list.len());
    if failed > 0 {
        log::error!("{failed} picked images could not be copied");
    }
    post_event(images_picked_event(list, failed));
    Ok(())
}

/// The user picked images; Kotlin starts copying them into the cache.
fn native_pick_copying<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> jni::errors::Result<()> {
    post_event(PlatformEvent::PickCopying);
    Ok(())
}

/// Kotlin could not start the photo picker or its I/O worker.
fn native_pick_failed<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> jni::errors::Result<()> {
    log::error!("photo picker failed");
    post_event(pick_failed_event());
    Ok(())
}

/// Kotlin finished (or failed) writing the collage to MediaStore.
fn native_saved<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    ok: bool,
) -> jni::errors::Result<()> {
    log::info!("saved to gallery: {ok}");
    post_event(saved_event(ok));
    Ok(())
}

/// Kotlin opened (or failed to open) the share sheet.
fn native_shared<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    ok: bool,
) -> jni::errors::Result<()> {
    log::info!("share sheet opened: {ok}");
    post_event(shared_event(ok));
    Ok(())
}
