//! SAF-backed progress files. Settings remain in the existing private directory.
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use jni::objects::{JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use crate::framework::error::{GameError, GameResult};
use crate::framework::vfs::{OpenOptions, PhysicalFS, VFile, VMetadata, VFS};
static FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn failed() -> bool { FAILED.load(std::sync::atomic::Ordering::Relaxed) }

fn android<T>(action: impl FnOnce(&JNIEnv, JObject) -> GameResult<T>) -> GameResult<T> {
    unsafe {
        let context = ndk_context::android_context();
        let vm = JavaVM::from_raw(context.vm().cast())?;
        let env = vm.attach_current_thread()?;
        let activity = JObject::from_raw(context.context().cast());
        let result = action(&env, activity);
        if env.exception_check()? { env.exception_clear()?; }
        if result.is_err() { FAILED.store(true, std::sync::atomic::Ordering::Relaxed); }
        result
    }
}

pub fn enabled() -> GameResult<bool> {
    android(|env, activity| Ok(env.call_method(activity, "hasPublicSaves", "()Z", &[])?.z()?))
}

pub fn open_migration(on_title: bool) -> GameResult {
    android(|env, activity| {
        env.call_method(activity, "openSaveMigration", "(Z)V", &[JValue::Bool(on_title as u8)])?;
        Ok(())
    })
}

pub fn migration_result() -> GameResult<i32> {
    android(|env, activity| Ok(env.call_method(activity, "saveMigrationResult", "()I", &[])?.i()?))
}

fn path_name(path: &Path) -> GameResult<String> {
    let mut parts = path.components();
    if !matches!(parts.next(), Some(Component::RootDir)) { return Err(error("Save path must be absolute")); }
    let mut names = Vec::new();
    for part in parts {
        match part {
            Component::Normal(name) => {
                let name = name.to_str().ok_or_else(|| error("Invalid save filename"))?;
                if name.contains(['\\', '\n', '\r', '\0']) || name.starts_with(".cavestory-") { return Err(error("Invalid save filename")); }
                names.push(name);
            }
            _ => return Err(error("Invalid save path")),
        }
    }
    Ok(names.join("/"))
}
fn error(message: &str) -> GameError { GameError::FilesystemError(message.into()) }
fn as_io(error: GameError) -> io::Error { io::Error::new(io::ErrorKind::Other, error.to_string()) }

fn read(path: &str) -> GameResult<Option<Vec<u8>>> {
    android(|env, activity| {
        let path = env.auto_local(env.new_string(path)?);
        let bytes = env.auto_local(env.call_method(activity, "readPublicSave", "(Ljava/lang/String;)[B", &[JValue::from(path.as_obj())])?.l()?);
        if bytes.as_obj().is_null() { return Ok(None); }
        Ok(Some(env.convert_byte_array(bytes.as_obj().into_raw())?))
    })
}
fn write(path: &str, bytes: &[u8]) -> GameResult {
    android(|env, activity| {
        let path = env.auto_local(env.new_string(path)?);
        let bytes = env.auto_local(unsafe { JObject::from_raw(env.byte_array_from_slice(bytes)?) });
        env.call_method(activity, "writePublicSave", "(Ljava/lang/String;[B)V", &[JValue::from(path.as_obj()), JValue::from(bytes.as_obj())])?;
        Ok(())
    })
}
fn stat(path: &str) -> GameResult<i64> {
    android(|env, activity| {
        let path = env.auto_local(env.new_string(path)?);
        Ok(env.call_method(activity, "statPublicSave", "(Ljava/lang/String;)J", &[JValue::from(path.as_obj())])?.j()?)
    })
}
fn change(method: &str, path: &str) -> GameResult {
    android(|env, activity| {
        let path = env.auto_local(env.new_string(path)?);
        env.call_method(activity, method, "(Ljava/lang/String;)V", &[JValue::from(path.as_obj())])?;
        Ok(())
    })
}

#[derive(Debug)]
struct SaveFile { path: String, data: Cursor<Vec<u8>>, writable: bool, append: bool, dirty: bool }
impl Read for SaveFile { fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.data.read(buf) } }
impl Seek for SaveFile { fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> { self.data.seek(pos) } }
impl Write for SaveFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.writable { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Read-only save")); }
        if self.append { self.data.set_position(self.data.get_ref().len() as u64); }
        if self.data.position().saturating_add(bytes.len() as u64) > 32 * 1024 * 1024 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Save exceeds size limit"));
        }
        self.dirty = true; self.data.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.dirty { write(&self.path, self.data.get_ref()).map_err(as_io)?; self.dirty = false; }
        Ok(())
    }
}
// Deliberately no Drop commit: callers must flush and observe its result.

#[derive(Debug)]
pub struct PublicSaves { private: PhysicalFS }
impl PublicSaves { pub fn new(private: &Path) -> Self { Self { private: PhysicalFS::new(private, false) } } }
struct Metadata(i64);
impl VMetadata for Metadata {
    fn is_dir(&self) -> bool { self.0 == -2 }
    fn is_file(&self) -> bool { self.0 >= 0 }
    fn len(&self) -> u64 { self.0.max(0) as u64 }
}
impl VFS for PublicSaves {
    fn open_options(&self, path: &Path, options: OpenOptions) -> GameResult<Box<dyn VFile>> {
        let name = path_name(path)?;
        if name == "settings.json" { return self.private.open_options(path, options); }
        let previous = read(&name)?;
        if previous.is_none() && !options.create { return Err(error("Save does not exist")); }
        let dirty = options.write && (options.truncate || previous.is_none());
        let bytes = if options.truncate { Vec::new() } else { previous.unwrap_or_default() };
        let mut data = Cursor::new(bytes);
        if options.append { data.set_position(data.get_ref().len() as u64); }
        Ok(Box::new(SaveFile { path: name, data, writable: options.write || options.append, append: options.append, dirty }))
    }
    fn mkdir(&self, path: &Path) -> GameResult { change("mkdirPublicSave", &path_name(path)?) }
    fn rm(&self, path: &Path) -> GameResult {
        let name = path_name(path)?;
        if name == "settings.json" { return self.private.rm(path); }
        change("deletePublicSave", &name)
    }
    fn rmrf(&self, _path: &Path) -> GameResult { Err(error("Recursive deletion of public saves is disabled")) }
    fn exists(&self, path: &Path) -> bool { self.metadata(path).is_ok() }
    fn metadata(&self, path: &Path) -> GameResult<Box<dyn VMetadata>> {
        let name = path_name(path)?;
        if name == "settings.json" { return self.private.metadata(path); }
        let info = stat(&name)?;
        if info == -1 { return Err(error("Save does not exist")); }
        Ok(Box::new(Metadata(info)))
    }
    fn read_dir(&self, path: &Path) -> GameResult<Box<dyn Iterator<Item = GameResult<PathBuf>>>> {
        let name = path_name(path)?;
        let listing = android(|env, activity| {
            let name = env.auto_local(env.new_string(&name)?);
            let result = env.auto_local(env.call_method(activity, "listPublicSaves", "(Ljava/lang/String;)Ljava/lang/String;", &[JValue::from(name.as_obj())])?.l()?);
            let text: String = env.get_string(JString::from(result.as_obj()))?.into(); Ok(text)
        })?;
        let entries: Vec<_> = listing.lines().filter(|name| *name != "settings.json").map(|name| Ok(path.join(name))).collect();
        Ok(Box::new(entries.into_iter()))
    }
    fn to_path_buf(&self) -> Option<PathBuf> { Some(PathBuf::from("android-public-saves")) }
}
