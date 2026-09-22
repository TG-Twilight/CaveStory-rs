use serde::{Deserialize, Serialize};
use std::{
    io::{self, Cursor},
    mem::size_of,
    ptr::{null, null_mut},
    thread,
    time::{Duration, Instant},
};
use trainer_bridge::{Bridge, GamePort};
use trainer_protocol::*;
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::{Pipes::*, SystemInformation::GetTickCount64, Threading::*},
};

const BUFFER: u32 = (MAX_FRAME_BYTES + 4) as u32;
const TIMEOUT: Duration = Duration::from_secs(2);
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct Local(*mut core::ffi::c_void);
impl Drop for Local {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Greeting {
    version: u16,
    instance: Id,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimedRequest {
    sent_at_ms: u64,
    request: Request,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn pipe_path(name: &str) -> io::Result<Vec<u16>> {
    if name.is_empty()
        || name.len() > 100
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid pipe name",
        ));
    }
    Ok(wide(&format!(r"\\.\pipe\{name}")))
}
pub fn name_for_pid(pid: u32) -> String {
    format!("CaveStory-rs-Trainer-{pid}")
}

// Protected DACL: only the current user SID; no inherited or Everyone ACE.
fn current_user_security() -> io::Result<Local> {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(io::Error::last_os_error());
        }
        let token = Handle(token);
        let mut needed = 0;
        GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut needed);
        if needed == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut data = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        if GetTokenInformation(
            token.0,
            TokenUser,
            data.as_mut_ptr().cast(),
            needed,
            &mut needed,
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let user = &*data.as_ptr().cast::<TOKEN_USER>();
        let mut sid = null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(io::Error::last_os_error());
        }
        let sid_owner = Local(sid.cast());
        let mut length = 0;
        while *sid.add(length) != 0 {
            length += 1;
        }
        let sid_text = String::from_utf16_lossy(std::slice::from_raw_parts(sid, length));
        drop(sid_owner);
        let descriptor_text = wide(&format!("D:P(A;;GA;;;{sid_text})"));
        let mut descriptor = null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            descriptor_text.as_ptr(),
            1,
            &mut descriptor,
            null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Local(descriptor))
    }
}

fn send<T: Serialize>(handle: &Handle, value: &T) -> io::Result<()> {
    let mut frame = vec![];
    write_frame(&mut frame, value)?;
    let mut written = 0;
    // All handles are synchronous PIPE_NOWAIT. One frame is one message.
    if unsafe {
        WriteFile(
            handle.0,
            frame.as_ptr(),
            frame.len() as u32,
            &mut written,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    if written != frame.len() as u32 {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "pipe output buffer full",
        ));
    }
    Ok(())
}
fn receive<T: serde::de::DeserializeOwned>(handle: &Handle) -> io::Result<Option<T>> {
    let mut bytes = vec![0; BUFFER as usize];
    let mut count = 0;
    if unsafe { ReadFile(handle.0, bytes.as_mut_ptr(), BUFFER, &mut count, null_mut()) } == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_NO_DATA as i32) {
            Ok(None)
        } else {
            Err(error)
        };
    }
    bytes.truncate(count as usize);
    let mut cursor = Cursor::new(&bytes);
    let result = read_frame(&mut cursor)?;
    if cursor.position() != bytes.len() as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "trailing bytes in pipe message",
        ));
    }
    Ok(Some(result))
}
fn wait<T: serde::de::DeserializeOwned>(handle: &Handle) -> io::Result<T> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(value) = receive(handle)? {
            return Ok(value);
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "game did not respond; outcome may be unknown",
            ));
        }
        thread::sleep(Duration::from_millis(5));
    }
}

pub struct Server {
    handle: Handle,
    connected: bool,
    last_activity: Instant,
}
impl Server {
    pub fn new(name: &str) -> io::Result<Self> {
        let path = pipe_path(name)?;
        let security = current_user_security()?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.0,
            bInheritHandle: 0,
        };
        let raw = unsafe {
            CreateNamedPipeW(
                path.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_MESSAGE
                    | PIPE_READMODE_MESSAGE
                    | PIPE_NOWAIT
                    | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                BUFFER,
                BUFFER,
                0,
                &attributes,
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            handle: Handle(raw),
            connected: false,
            last_activity: Instant::now(),
        })
    }
    pub fn poll<G: GamePort>(&mut self, bridge: &mut Bridge<G>) -> io::Result<usize> {
        let now = Instant::now();
        bridge.expire(now);
        let result = self.poll_inner(bridge, now);
        if result.is_err()
            || (self.connected && now.duration_since(self.last_activity) >= trainer_bridge::LEASE)
        {
            unsafe {
                DisconnectNamedPipe(self.handle.0);
            }
            self.connected = false;
            bridge.disconnect();
        }
        result
    }
    fn poll_inner<G: GamePort>(
        &mut self,
        bridge: &mut Bridge<G>,
        now: Instant,
    ) -> io::Result<usize> {
        if !self.connected {
            let ok = unsafe { ConnectNamedPipe(self.handle.0, null_mut()) };
            if ok == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_PIPE_LISTENING as i32) {
                    return Ok(0);
                }
                if error.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
                    return Err(error);
                }
            }
            let mut client_pid = 0;
            if unsafe { GetNamedPipeClientProcessId(self.handle.0, &mut client_pid) } == 0
                || client_pid == 0
            {
                return Ok(0);
            }
            send(
                &self.handle,
                &Greeting {
                    version: VERSION,
                    instance: bridge.instance(),
                },
            )?;
            self.connected = true;
            self.last_activity = now;
        }
        let mut processed = 0;
        for _ in 0..4 {
            let Some(timed) = receive::<TimedRequest>(&self.handle)? else {
                break;
            };
            self.last_activity = now;
            // Windows uptime is shared across processes. Include time in the OS buffer,
            // not just time after the game thread finally receives the message.
            let age = unsafe { GetTickCount64() }.checked_sub(timed.sent_at_ms);
            let response = if age.is_none_or(|ms| ms >= 500) {
                bridge.reject(timed.request.request_id, ErrorCode::Expired)
            } else {
                bridge.handle(timed.request, now)
            };
            send(&self.handle, &response)?;
            processed += 1;
        }
        Ok(processed)
    }
}

pub struct Connection {
    handle: Option<Handle>,
}
impl Connection {
    pub fn open(name: &str, pid: u32) -> io::Result<(Self, Id)> {
        let path = pipe_path(name)?;
        let deadline = Instant::now() + TIMEOUT;
        let raw = loop {
            let raw = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    null(),
                    OPEN_EXISTING,
                    SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                    null_mut(),
                )
            };
            if raw != INVALID_HANDLE_VALUE {
                break raw;
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) || Instant::now() >= deadline {
                return Err(error);
            }
            thread::sleep(Duration::from_millis(5));
        };
        let handle = Handle(raw);
        let mut actual_pid = 0;
        if unsafe { GetNamedPipeServerProcessId(handle.0, &mut actual_pid) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if actual_pid != pid {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "server PID mismatch",
            ));
        }
        let mode = PIPE_READMODE_MESSAGE | PIPE_NOWAIT;
        if unsafe { SetNamedPipeHandleState(handle.0, &mode, null(), null()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let greeting: Greeting = wait(&handle)?;
        if greeting.version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "protocol version mismatch",
            ));
        }
        Ok((
            Self {
                handle: Some(handle),
            },
            greeting.instance,
        ))
    }
    pub fn exchange(&mut self, request: &Request) -> io::Result<Response> {
        let handle = self.handle.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                "connection closed after transport error",
            )
        })?;
        let timed = TimedRequest {
            sent_at_ms: unsafe { GetTickCount64() },
            request: request.clone(),
        };
        let result = send(handle, &timed).and_then(|()| wait(handle));
        if result.is_err() {
            self.handle = None;
        }
        result
    }
}
