use std::env;

#[path = "src/build_version.rs"]
mod build_version;

#[cfg(target_os = "windows")]
extern crate winres;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(trainer_interface)");
    if matches!(env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("windows" | "android"))
        || env::var_os("CARGO_FEATURE_TRAINER_CORE").is_some()
    {
        println!("cargo:rustc-cfg=trainer_interface");
    }
    // let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap_or_else(|e| panic!("{}", e));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/build_version.rs");
    println!("cargo:rerun-if-env-changed=CAVESTORY_BUILD_VERSION");
    let version = env::var("CAVESTORY_BUILD_VERSION").unwrap_or_else(|_| {
        let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 86400;
        build_version::date_from_unix_days(days)
    });
    let _numeric_version = build_version::windows_version(&version);
    println!("cargo:rustc-env=CAVESTORY_BUILD_VERSION={version}");

    #[cfg(target_os = "windows")]
    if target.contains("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("res/crabsue-icon.ico");
        res.set("ProductName", "CaveStory-rs");
        res.set("FileDescription", "CaveStory-rs");
        res.set("InternalName", "CaveStory-rs");
        res.set("OriginalFilename", "CaveStory-rs.exe");
        res.set("ProductVersion", &version);
        res.set("FileVersion", &version);
        res.set_version_info(winres::VersionInfo::PRODUCTVERSION, _numeric_version);
        res.set_version_info(winres::VersionInfo::FILEVERSION, _numeric_version);
        res.compile().unwrap();

        if target.contains("i686") {
            // hack
            println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
            println!("cargo:rustc-link-lib=shlwapi");
        }
    }

    if target.contains("darwin") {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15");
        println!("cargo:rustc-link-arg=-weak_framework");
        println!("cargo:rustc-link-arg=GameController");
        println!("cargo:rustc-link-arg=-weak_framework");
        println!("cargo:rustc-link-arg=CoreHaptics");
    }

    if target.contains("android") {
        println!("cargo:rustc-link-lib=dylib=GLESv2");
        println!("cargo:rustc-link-lib=dylib=EGL");
    }
}
