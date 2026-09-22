//! Cached Android input inventory (including keyboards and mice SDL does not enumerate).
use crate::framework::error::GameResult;
use jni::objects::JObject;
use jni::JavaVM;

pub fn external_input_capabilities() -> GameResult<i32> {
    unsafe {
        let context = ndk_context::android_context();
        let vm = JavaVM::from_raw(context.vm().cast())?;
        let env = vm.attach_current_thread()?;
        let activity = JObject::from_raw(context.context().cast());
        let result = env.call_method(activity, "getExternalInputCapabilities", "()I", &[]);
        if env.exception_check()? { env.exception_clear()?; }
        Ok(result?.i()?)
    }
}
