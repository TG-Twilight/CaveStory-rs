//! Reads the application/system language through Android's public locale APIs.
use crate::framework::error::GameResult;
use jni::objects::{JObject, JString, JValue};
use jni::JavaVM;

pub fn language() -> GameResult<String> {
    unsafe {
        let context = ndk_context::android_context();
        let vm = JavaVM::from_raw(context.vm().cast())?;
        let env = vm.attach_current_thread()?;
        let activity = JObject::from_raw(context.context().cast());
        let result = env.call_method(activity, "getGameLanguage", "()Ljava/lang/String;", &[]);
        if env.exception_check()? { env.exception_clear()?; }
        // SDL's nativeRunMain remains on this Java thread for the whole game.
        // Periodic reads must release JNI locals instead of waiting for its return.
        let value = env.auto_local(result?.l()?);
        let text: String = env.get_string(JString::from(value.as_obj()))?.into();
        Ok(text)
    }
}

pub fn show_data_error(detail: &str) -> GameResult {
    unsafe {
        let context = ndk_context::android_context();
        let vm = JavaVM::from_raw(context.vm().cast())?;
        let env = vm.attach_current_thread()?;
        let activity = JObject::from_raw(context.context().cast());
        let detail = env.auto_local(env.new_string(detail)?);
        let result = env.call_method(activity, "showDataError", "(Ljava/lang/String;)V", &[JValue::from(detail.as_obj())]);
        if env.exception_check()? { env.exception_clear()?; }
        result?;
        Ok(())
    }
}
