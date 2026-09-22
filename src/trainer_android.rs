//! JNI queues only; all engine state remains on the SDL game thread.
use jni::{JNIEnv, objects::{JClass, JString}, sys::{jboolean, jlong, jstring}};
use std::{sync::{Mutex, OnceLock, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use trainer_bridge::{channel::Channel, Bridge, GamePort};

static ENABLED: AtomicBool = AtomicBool::new(false);
static CURRENT: Mutex<Option<Channel>> = Mutex::new(None);
static CLOCK: OnceLock<(Instant, u64)> = OnceLock::new();

pub fn enabled() -> bool { ENABLED.load(Ordering::Acquire) }
fn elapsed_ms() -> u64 {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // CLOCK_BOOTTIME matches SystemClock.elapsedRealtime, including deep sleep.
    let result = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time) };
    assert_eq!(result, 0, "CLOCK_BOOTTIME unavailable");
    (time.tv_sec as u64).saturating_mul(1000) + time.tv_nsec as u64 / 1_000_000
}
fn lease_now() -> Instant {
    let (origin, elapsed) = CLOCK.get_or_init(|| (Instant::now(), elapsed_ms()));
    *origin + Duration::from_millis(elapsed_ms().saturating_sub(*elapsed))
}
pub struct Endpoint(Channel);
impl Endpoint {
    pub fn open() -> std::io::Result<Self> {
        let mut current = CURRENT.lock().unwrap();
        if !enabled() { return Err(std::io::ErrorKind::PermissionDenied.into()); }
        let channel = Channel::new();
        if let Some(old) = current.replace(channel.clone()) { old.shutdown(); }
        Ok(Self(channel))
    }
    pub fn poll<G: GamePort>(&mut self, bridge: &mut Bridge<G>) -> std::io::Result<usize> {
        bridge.set_enabled(enabled());
        Ok(self.0.tick(bridge, lease_now(), elapsed_ms))
    }
}
impl Drop for Endpoint { fn drop(&mut self) { self.0.shutdown(); } }

fn channel() -> Result<Channel, String> {
    let current = CURRENT.lock().unwrap();
    if !enabled() { return Err("Trainer access disabled".into()); }
    current.clone().ok_or_else(|| "Enter a game scene first".into())
}
fn output(env: &JNIEnv, result: Result<Option<String>, String>) -> jstring {
    match result {
        Ok(Some(text)) => match env.new_string(text) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Ok(None) => std::ptr::null_mut(),
        Err(error) => { let _ = env.throw_new("java/lang/IllegalStateException", error); std::ptr::null_mut() }
    }
}
#[no_mangle]
pub extern "system" fn Java_io_github_cavestory_1rs_TrainerBridge_setEnabled(_env: JNIEnv, _class: JClass, value: jboolean) {
    let current = CURRENT.lock().unwrap();
    ENABLED.store(value != 0, Ordering::Release);
    if value == 0 { if let Some(channel) = current.as_ref() { channel.invalidate(); } }
}
#[no_mangle]
pub extern "system" fn Java_io_github_cavestory_1rs_TrainerBridge_open(env: JNIEnv, _class: JClass, connection: jlong) -> jstring {
    let result = (|| {
        if connection <= 0 { return Err("Invalid connection".into()); }
        // Waiting for a scene is not a successful editable connection.
        let Ok(channel) = channel() else { return Ok(None); };
        match channel.open(connection as u64) {
            Ok(Some(instance)) => serde_json::to_string(&instance).map(Some).map_err(|e| e.to_string()),
            Ok(None) | Err(_) => Ok(None),
        }
    })();
    output(&env,result)
}
#[no_mangle]
pub extern "system" fn Java_io_github_cavestory_1rs_TrainerBridge_exchange(env: JNIEnv, _class: JClass, connection: jlong, sent: jlong, text: JString) -> jstring {
    let result = (|| {
        if connection <= 0 || sent < 0 { return Err("Invalid transport metadata".into()); }
        let text: String = env.get_string(text).map_err(|e| e.to_string())?.into();
        if text.is_empty() || text.len() > trainer_protocol::MAX_FRAME_BYTES { return Err("Frame too large".into()); }
        let request = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let pending = channel()?.submit(connection as u64, sent as u64, elapsed_ms(), request).map_err(|e| format!("{e:?}"))?;
        let response = pending.wait(Duration::from_millis(650)).map_err(|e| format!("{e:?}; sent write outcome may be unknown"))?;
        let encoded = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        if encoded.len() > trainer_protocol::MAX_FRAME_BYTES { return Err("Response too large".into()); }
        Ok(Some(encoded))
    })();
    output(&env,result)
}
#[no_mangle]
pub extern "system" fn Java_io_github_cavestory_1rs_TrainerBridge_close(_env: JNIEnv, _class: JClass, connection: jlong) {
    if let Some(channel) = CURRENT.lock().unwrap().as_ref() { channel.close(connection as u64); }
}
