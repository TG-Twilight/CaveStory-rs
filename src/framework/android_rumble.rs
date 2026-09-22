//! Android activity bridge. Bluetooth and Binder work stays outside the SDL thread.
use crate::framework::error::GameResult;
use jni::objects::{JObject, JValue};
use jni::JavaVM;

pub fn show_settings(player: i32, instance: Option<u32>, allowed: bool, ok: i32, back: i32) -> GameResult {
    extern "C" { fn CaveStory_AndroidDeviceId(instance: i32) -> i32; }
    unsafe {
        let context = ndk_context::android_context();
        let vm = JavaVM::from_raw(context.vm().cast())?;
        let env = vm.attach_current_thread()?;
        let activity = JObject::from_raw(context.context().cast());
        let instance = instance.map(|value| value as i32).unwrap_or(-1);
        let device = if instance >= 0 { CaveStory_AndroidDeviceId(instance) } else { -1 };
        let result = env.call_method(activity, "showShizukuRumbleSettings", "(IIIZII)V", &[
            JValue::Int(player), JValue::Int(instance), JValue::Int(device), JValue::Bool(allowed as u8),
            JValue::Int(ok), JValue::Int(back),
        ]);
        if env.exception_check()? { env.exception_clear()?; }
        result?;
        Ok(())
    }
}

pub fn rumble(instance: u32, low: u16, high: u16, ms: u32) -> bool {
    fn invoke(instance: u32, low: u16, high: u16, ms: u32) -> GameResult<bool> {
        extern "C" { fn CaveStory_AndroidDeviceId(instance: i32) -> i32; }
        unsafe {
            let device = CaveStory_AndroidDeviceId(instance as i32);
            // HIDAPI/USB controllers are not in the Android driver list.
            if device < 0 { return Ok(false); }
            let context = ndk_context::android_context();
            let vm = JavaVM::from_raw(context.vm().cast())?;
            let env = vm.attach_current_thread()?;
            let activity = JObject::from_raw(context.context().cast());
            let result = env.call_method(activity, "shizukuRumble", "(IIII)Z", &[
                JValue::Int(device), JValue::Int(low as i32), JValue::Int(high as i32),
                JValue::Int(ms.min(2500) as i32),
            ]);
            if env.exception_check()? { env.exception_clear()?; }
            Ok(result?.z()?)
        }
    }
    invoke(instance, low, high, ms).unwrap_or(false)
}

pub fn prefers_bluetooth(instance: u32) -> bool {
    fn invoke(instance: u32) -> GameResult<bool> {
        extern "C" { fn CaveStory_AndroidDeviceId(instance: i32) -> i32; }
        unsafe {
            let device = CaveStory_AndroidDeviceId(instance as i32);
            if device < 0 { return Ok(false); }
            let context = ndk_context::android_context();
            let vm = JavaVM::from_raw(context.vm().cast())?;
            let env = vm.attach_current_thread()?;
            let activity = JObject::from_raw(context.context().cast());
            let result = env.call_method(activity, "prefersBluetoothRumble", "(I)Z", &[JValue::Int(device)]);
            if env.exception_check()? { env.exception_clear()?; }
            Ok(result?.z()?)
        }
    }
    invoke(instance).unwrap_or(false)
}
