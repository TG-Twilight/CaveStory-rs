//! Local validation harness: normal engine + process-local SDL virtual Xbox pad.
//! Requires CAVESTORY_FEEDBACK_PROBE pointing to an existing disposable directory.
use clap::Parser;
use sdl2_sys as sdl;
use std::ffi::{c_void, CString};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Capture { start: Instant, file: Mutex<File> }

unsafe extern "C" fn rumble(data: *mut c_void, low: u16, high: u16) -> i32 {
    let capture = &*(data as *const Capture);
    if let Ok(mut file) = capture.file.lock() {
        let _ = writeln!(file, "{{\"elapsed_ms\":{},\"low\":{},\"high\":{}}}", capture.start.elapsed().as_millis(), low, high);
        let _ = file.flush();
    }
    0
}

unsafe fn attach(capture: *mut Capture) -> Result<*mut sdl::SDL_Joystick, String> {
    let name = CString::new("CaveStory Feedback Virtual Xbox Controller").unwrap();
    let mut desc: sdl::SDL_VirtualJoystickDesc = std::mem::zeroed();
    desc.version = sdl::SDL_VIRTUAL_JOYSTICK_DESC_VERSION as u16;
    desc.type_ = sdl::SDL_JoystickType::SDL_JOYSTICK_TYPE_GAMECONTROLLER as u16;
    desc.naxes = 6;
    desc.nbuttons = 15;
    desc.vendor_id = 0x045e;
    desc.product_id = 0x028e;
    desc.axis_mask = 0x3f;
    desc.button_mask = 0x7fff;
    desc.name = name.as_ptr();
    desc.userdata = capture.cast();
    desc.Rumble = Some(rumble);
    let index = sdl::SDL_JoystickAttachVirtualEx(&desc);
    if index < 0 { return Err(sdl_error()); }
    let pad = sdl::SDL_JoystickOpen(index);
    if pad.is_null() { return Err(sdl_error()); }
    sdl::SDL_JoystickSetVirtualAxis(pad, 4, i16::MIN);
    sdl::SDL_JoystickSetVirtualAxis(pad, 5, i16::MIN);
    Ok(pad)
}

unsafe fn sdl_error() -> String {
    std::ffi::CStr::from_ptr(sdl::SDL_GetError()).to_string_lossy().into_owned()
}

fn serve(folder: PathBuf, capture: &'static Capture) -> Result<(), String> {
    unsafe {
        while sdl::SDL_WasInit(sdl::SDL_INIT_JOYSTICK) == 0 {
            std::thread::sleep(Duration::from_millis(20));
        }
        sdl::SDL_LockJoysticks();
        let initial = attach(capture as *const Capture as *mut Capture);
        sdl::SDL_UnlockJoysticks();
        let mut pad = initial?;
        fs::write(folder.join("ready.txt"), "ready\n").map_err(|e| e.to_string())?;
        let mut last = 0u64;
        loop {
            if let Ok(text) = fs::read_to_string(folder.join("command.txt")) {
                let args: Vec<_> = text.split_whitespace().collect();
                if let Some(seq) = args.first().and_then(|v| v.parse::<u64>().ok()).filter(|n| *n > last) {
                    // Only complete newline-terminated commands are consumed.
                    if !text.ends_with('\n') { continue; }
                    last = seq;
                    sdl::SDL_LockJoysticks();
                    let result = match args.get(1).copied() {
                        Some("button") if args.len() == 4 && !pad.is_null() => {
                            match (args[2].parse::<i32>(), args[3].parse::<u8>()) {
                                (Ok(n), Ok(value)) if (0..15).contains(&n) && value <= 1 => sdl::SDL_JoystickSetVirtualButton(pad, n, value),
                                _ => -2,
                            }
                        }
                        Some("axis") if args.len() == 4 && !pad.is_null() => {
                            match (args[2].parse::<i32>(), args[3].parse::<i16>()) {
                                (Ok(n), Ok(value)) if (0..6).contains(&n) => sdl::SDL_JoystickSetVirtualAxis(pad, n, value),
                                _ => -2,
                            }
                        }
                        Some("detach") if !pad.is_null() => {
                            let id = sdl::SDL_JoystickInstanceID(pad);
                            let index = (0..sdl::SDL_NumJoysticks()).find(|i| sdl::SDL_JoystickGetDeviceInstanceID(*i) == id);
                            sdl::SDL_JoystickClose(pad);
                            pad = std::ptr::null_mut();
                            index.map_or(-2, |i| sdl::SDL_JoystickDetachVirtual(i))
                        }
                        Some("attach") if pad.is_null() => {
                            match attach(capture as *const Capture as *mut Capture) {
                                Ok(new_pad) => { pad = new_pad; 0 }
                                Err(_) => -1,
                            }
                        }
                        _ => -2,
                    };
                    sdl::SDL_UnlockJoysticks();
                    fs::write(folder.join("ack.txt"), format!("{} {}\n", seq, result)).map_err(|e| e.to_string())?;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn main() {
    let folder = std::env::var_os("CAVESTORY_FEEDBACK_PROBE").map(PathBuf::from)
        .expect("Set CAVESTORY_FEEDBACK_PROBE to an existing disposable probe folder");
    assert!(folder.is_dir(), "Probe directory must already exist");
    let file = OpenOptions::new().write(true).create_new(true).open(folder.join("rumble.jsonl"))
        .expect("Use a fresh folder: rumble.jsonl must not exist");
    let capture = Box::leak(Box::new(Capture { start: Instant::now(), file: Mutex::new(file) }));
    std::thread::spawn(move || {
        if let Err(error) = serve(folder.clone(), capture) {
            let _ = fs::write(folder.join("error.txt"), error);
        }
    });
    if let Err(error) = doukutsu_rs::game::init(doukutsu_rs::game::LaunchOptions::parse()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
