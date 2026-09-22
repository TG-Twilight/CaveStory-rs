<p align="center">
  <img src="res/readme/hero.png" alt="CaveStory-rs" width="100%">
</p>

<p align="center">
  <strong>Back to the caves. On your terms.</strong><br>
  Three languages. Your controller. Windows &amp; Android.
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="docs/README_zh-CN.md">简体中文</a> · <a href="docs/README_ja-JP.md">日本語</a>
</p>

<p align="center">
  <img src="res/readme/rust.svg" alt="Rust" height="24">
  <img src="res/readme/windows.svg" alt="Windows" height="24">
  <img src="res/readme/android.svg" alt="Android" height="24">
  <a href="LICENSE"><img src="res/readme/license.svg" alt="Engine license: MIT" height="24"></a>
</p>

<p align="center">
  <a href="#features">Features</a> · <a href="#screenshots">Screenshots</a> · <a href="#play">Play</a> · <a href="#build">Build</a> · <a href="#credits">Credits</a>
</p>

CaveStory-rs is a community fork of [doukutsu-rs](https://github.com/doukutsu-rs/doukutsu-rs), a Cave Story engine reimplementation written in Rust. It brings together Simplified Chinese, English, and Japanese game resources with improvements to controllers, Android setup, and save management. Current builds focus on Windows and Android and the original 2004 freeware game.

<a id="features"></a>

## Made for the way you play

<table>
  <tr>
    <td width="50%" valign="top"><h3>🌐 Three languages, throughout</h3><p>Menus, dialogue, items, maps, credits, and image text. Switch between English, 简体中文, and 日本語.</p></td>
    <td width="50%" valign="top"><h3>🎮 Our own controller experience</h3><p>Xbox / PSP layouts, custom bindings, automatic assignment, hot-plug handling, and enhanced vibration.</p></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><h3>📳 Shizuku Bluetooth vibration</h3><p>Our Android output path brings physical controller vibration to supported setups through Shizuku authorization.</p></td>
    <td width="50%" valign="top"><h3>📱 Open it and start playing</h3><p>Offline resource setup in the game-data APK. System-language integration and touch buttons that respond to peripherals.</p></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><h3>💾 Saves where you want them</h3><p>Private or public Android save folders, with verified migration that keeps your original files.</p></td>
    <td width="50%" valign="top"><h3>🧩 Ready for Trainer</h3><p>An optional local interface for the separate CaveStory-rs Trainer. You choose when to enable it.</p></td>
  </tr>
</table>

### A little extra for your controller

**Bluetooth controller vibration through Shizuku.** We developed an Android output path that uses Shizuku-authorized Shell services to send vibration commands to supported Bluetooth controllers when ordinary application-level output is unavailable. It is an original contribution we are particularly proud of (and we think it might be a first of its kind). This includes player-to-device matching, bounded output, authorization and reconnection handling, and stopping vibration when the game goes into the background. Physical vibration has been confirmed on tested setups, including a non-rooted Xiaomi Mi 9; compatibility still depends on the controller, connection, and OS. The feature is optional and ordinary play does not require Shizuku.

<details>
<summary><strong>Explore the feature details</strong></summary>

- **Three languages across menus and game resources.** Simplified Chinese, English, and Japanese selection switches dialogue, item descriptions, map names, credits, and relevant image text along with the interface. Separate resource directories, encoding support, and Chinese/Japanese pixel fonts keep the languages usable together. Full playthrough proofreading is still pending.
- **Our own controller integration and vibration design.** This project implements automatic assignment and hot-plug improvements, Xbox/PSP button-layout presets, persistent custom bindings, and enhanced gameplay vibration. Keyboard, touch input, and player-specific device assignments are preserved. Vibration defaults to enabled with the enhanced effect scheme, while respecting saved preferences. Android separates the effect scheme from the system/Bluetooth output method and provides vibration previews.
- **Touch controls that respond to peripherals.** Android hides touch buttons when a controller connects and restores them when no relevant external input device remains. An optional 10-second touch-idle hiding setting is off by default; touching restores the display, and holding a control does not hide it.
- **Simpler Android startup.** The package with game data prepares its resources offline on first launch. The base package asks to download missing resources, and cancellation exits normally. Native setup and error messages support all three languages, including before game fonts are available.
- **Android language integration.** New installations follow the system/app language: Simplified Chinese locales use Simplified Chinese, Japanese uses Japanese, and other locales currently use English. Manual in-game choices persist; a later system/app language change takes effect again. Changes received during play are deferred until the title screen to preserve the current session.
- **Save location choice and migration.** Android can keep saves privately or in a selected local public folder. Public storage contains saves, replays, and progress records; game resources and device settings stay private. Migration in the advanced options copies and verifies files before switching, keeps the originals, and handles conflicts explicitly. Returning through the launcher also preserves an existing game session.
- **Optional local Trainer integration.** Windows and Android builds include the game-side interface for the separately maintained CaveStory-rs Trainer. Windows requires explicit activation, such as `CAVESTORY_TRAINER=1`; Android requires the user's authorization through a Trainer signed with the same certificate. Connections are disabled by default. The game builds independently from the vendored interface sources; the Trainer application is separate.
- **Consistent packages and distribution notices.** Five architectures each have a base package and a package with game data, with architecture checks, source records, and SHA256 manifests. Android includes a local-only source questionnaire and a release-signature check before starting the native game. In-game project links open only after confirmation. These checks discourage simple repackaging; they cannot prove the download website or prevent all modifications.

</details>

<a id="screenshots"></a>

## A look inside

Actual Android captures from our validation sessions. Click an image to view it at full size.

<table>
  <tr>
    <td width="50%"><a href="res/readme/languages.png"><img src="res/readme/languages.png" alt="Three languages in one installation" width="100%"></a><br><sub>Three languages in one installation</sub></td>
    <td width="50%"><a href="res/readme/japanese-dialogue.png"><img src="res/readme/japanese-dialogue.png" alt="Japanese dialogue, in game" width="100%"></a><br><sub>Japanese dialogue, in game</sub></td>
  </tr>
  <tr>
    <td width="50%"><a href="res/readme/android-gameplay.png"><img src="res/readme/android-gameplay.png" alt="Touch controls on Android" width="100%"></a><br><sub>Touch controls on Android</sub></td>
    <td width="50%"><a href="res/readme/controller-options.png"><img src="res/readme/controller-options.png" alt="Our controller and vibration options" width="100%"></a><br><sub>Our controller and vibration options</sub></td>
  </tr>
</table>

<a id="about"></a>

## Why this project exists

The motivation is simple: to enjoy Cave Story on the devices we use today, in a language we can comfortably read, with controls that feel convenient on both a computer and a phone.

doukutsu-rs provides the foundation for that goal. This fork began with the missing pieces of a complete Simplified Chinese experience: menus alone were not enough; dialogue, item descriptions, map names, fonts, and image text needed to work together. It has since grown to cover language switching, easier resource setup, controller handling, vibration, and Android save storage.

<a id="comparison"></a>

## What is different?

### Compared with the original 2004 PC game

The original Cave Story is Daisuke Amaya's game, released under the name Pixel. CaveStory-rs runs its game data through the Rust engine developed by doukutsu-rs. It builds on that engine's support for modern systems, display options, configurable input, and Android touch controls.

The focus remains the freeware adventure, its pixel art, music, and gameplay. This fork adds multilingual resource handling and platform conveniences. It also adds a short distribution reminder to a known early Balrog dialogue, so the supplied scripts include a small, deliberate addition to the original text. The original scene and battle choices remain, and original scripts are backed up before patching.

Our current adaptation and validation cover the freeware data and its translations. Upstream's support for other editions and platforms does not mean this fork has validated Cave Story+, commercial remasters, or every upstream port.

### Compared with doukutsu-rs

This fork brings together complete three-language resource switching, our own controller and vibration work, Shizuku Bluetooth output, Android startup and save management, and the optional Trainer interface. The [feature overview](#features) introduces these additions; the underlying Rust engine comes from doukutsu-rs.

<a id="play"></a>

## Getting started

The current local delivery matrix is:

| Platform | Architectures | Formats |
| --- | --- | --- |
| Windows | `x86_64` · `x86_32` · `arm64` | ZIP, base / `_game` |
| Android | `arm64-v8a` · `armeabi-v7a` | APK, base / `_game` |

- **With game data (`_game`):** three languages and fonts, plus the matching Windows runtime. Android prepares bundled resources offline.
- **Base:** supply compatible game resources and the matching Windows Visual C++ runtime; Android offers to download missing resources.

Files use `CaveStory-rs_<platform>_<YYYYMMDD>_<architecture>[_game].zip` or `.apk`. Android packages are separate ABI builds; choose one supported by your device's OS. The configured Android minimum is Android 7.0 / API 24, which is not a claim that every compatible device has been tested.

Download this fork from [GitHub Releases](https://github.com/TG-Twilight/CaveStory-rs/releases/latest). The current Release uses locally built and verified packages. [GitHub Actions](https://github.com/TG-Twilight/CaveStory-rs/actions/workflows/build.yml) builds the five-architecture, ten-package matrix and uploads a complete delivery artifact after package verification. These builds do not automatically publish a Release; see the [workflow guide](docs/CONTRIBUTING.md#github-actions-builds).

### Windows

Extract the entire package with game data to a writable folder and run `CaveStory-rs.exe`. It initially uses Simplified Chinese; select English or Japanese in **选项 → 语言** (Options → Language). Keep `user/` when upgrading or moving the installation, as it contains saves and settings. Keep the original `Doukutsu.exe` and the original executables in the language directories for resource extraction.

<details>
<summary><strong>Base package setup and adding languages</strong></summary>

For a base package, use a working copy of compatible freeware data. The tested Chinese setup merges the supplied `data/fonts` and `data/locale` into a copy of the Simplified Chinese game and places `CaveStory-rs.exe` beside `Doukutsu.exe`. Preserve existing saves and customized files.

Recognized managed Chinese installations can add English or Japanese resources with the following commands, substituting the actual `data` path. Existing language directories are protected from replacement:

```powershell
py tools/install_english_resources.py --data "C:/Games/CaveStory-rs/data"
py tools/install_japanese_resources.py --data "C:/Games/CaveStory-rs/data"
```

</details>

### Android

Install the APK for your ABI, complete resource setup, and choose a private or public save folder. Public folders must be local folders supported by the system picker; cloud storage and arbitrary document providers are not supported. Private files remain accessible through the game's document provider in a file manager's **Add storage** flow.

To move saves later, return to the title screen and open **Options → Advanced... → Save folder**. The original files are retained. Keep a separate backup when moving between installations or devices.

Base and game-data release APKs share `io.github.cavestory_rs` and the same signing identity, allowing in-place updates between them. The older `io.github.doukutsu_rs` app and debug apps have separate storage; their saves are not imported automatically.

### Game resources

The resource entry point is the [Cave Story Tribute Site](https://www.cavestory.one/download/cave-story.php). The Simplified Chinese translation is credited to Hydrowing, the English translation to Aeon Genesis, and the Japanese archive comes from Studio Pixel. Packaging tools record the selected archives and hashes. Packages with game data are intended to simplify setup and reduce repeated downloads from the tribute site; complete original game data is kept out of the source repository and base packages.

Repository map and contribution guidelines: [Contributing](docs/CONTRIBUTING.md).

<a id="build"></a>

## Building from source

Choose a build path below. All outputs stay in the sibling CaveStory-rs-runs directory.

<details>
<summary><strong>Prepare the source and tools</strong></summary>

```sh
git clone --recurse-submodules https://github.com/TG-Twilight/CaveStory-rs.git
cd CaveStory-rs
git submodule update --init --recursive
```

Use Rust/Cargo, Git, a C/C++ toolchain, and CMake. [Cargo.toml](Cargo.toml) declares Rust 1.88 as the minimum; recorded local builds used Rust 1.98.1, so the minimum itself has not been validated. The default desktop backend builds SDL2 from source. Resource preparation additionally requires Python 3 and Pillow (`py -m pip install Pillow` on Windows).

Build output belongs in the sibling `CaveStory-rs-runs/` directory. [.cargo/config.toml](.cargo/config.toml) already directs Cargo there; downloads and packaging caches are also kept outside the source tree. The game-side Trainer dependencies are included in `vendor/trainer/`; no sibling Trainer checkout is needed.

</details>

<details>
<summary><strong>Windows engine build</strong></summary>

Install Visual Studio C++ Build Tools and a Windows SDK, and run from a developer shell configured for the target architecture. Ensure CMake is on `PATH`. For an x64 engine build:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release --locked --bin CaveStory-rs --target x86_64-pc-windows-msvc
```

The executable is `../CaveStory-rs-runs/cache/cargo/x86_64-pc-windows-msvc/release/CaveStory-rs.exe`. This compiles the engine; pair it with game resources as described above. The other Windows targets are `i686-pc-windows-msvc` and `aarch64-pc-windows-msvc`, each requiring its matching MSVC tools and libraries.

</details>

<details>
<summary><strong>Android build</strong></summary>

The current project uses JDK 17-compatible tooling, SDK 35, Build Tools `35.0.1`, NDK `28.0.13004108`, and CMake 3.22 or newer. The Gradle wrapper and Android Gradle Plugin versions are defined in the repository. Install both Rust targets:

```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi
```

**Signing must be configured before building.** Both Debug and Release currently require a JKS; there is no automatic debug-key fallback. Gradle accepts `REVIA_KS_PATH` and `REVIA_KS_PASS`, expects one private-key entry, and uses the same store/key password. The maintainer's private key is not included. For your own build, supply your own key and configure [SignaturePolicy.java](platforms/android/app/src/main/java/io/github/cavestory_rs/SignaturePolicy.java) for your certificate; otherwise the installed app will refuse to start. Clearly identify redistributed builds as your own.

Once signing variables and `JAVA_HOME` / `ANDROID_HOME` are set, run from the repository root:

```powershell
cd platforms/android
./gradlew.bat assembleDebug --project-cache-dir ../../../CaveStory-rs-runs/cache/gradle-project --console=plain
cd ../..
```

This generates two base debug APKs under `../CaveStory-rs-runs/cache/android/app/build/outputs/apk/debug/`, using the `.debug` application suffix. `assembleRelease` builds the release variant. To bundle verified game archives, first prepare them with `py tools/package_windows_with_game.py --android-assets <directory-inside-CaveStory-rs-runs>`, then pass that directory to Gradle as `-PcaveStoryGameAssets=<directory>`. Initial dependency, font, and game-archive preparation requires network access when caches are empty.

</details>

<details>
<summary><strong>Full local delivery: five architectures, ten packages</strong></summary>

The maintainer's build entry point is:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build_all.ps1
```

It creates Android's two ARM APKs and Windows' three architecture ZIPs, each with base and game-data variants, under `../CaveStory-rs-runs/builds/<batch>/`. All ten share a date version. The batch includes `build-info.json`, `SHA256SUMS`, and `verified-delivery.json`.

This is a Windows maintainer workflow. On another machine, supply `VSROOT` and `CAVESTORY_ARMTOOLS` for the MSVC installation and ARM64 compiler, and `JAVA_HOME` / `ANDROID_HOME` for Android tooling. [setup_ci.ps1](tools/setup_ci.ps1) shows the hosted-runner setup; the build scripts retain their original local defaults. Android reads `REVIA_KS_PASS` from the process environment, falling back to the Windows **user-level** variable, and accepts `REVIA_KS_PATH` for the JKS location. The APK verification scripts expect the maintainer's certificate fingerprint, so independent builds must update that expectation together with the app's signing policy.

Prepare the pinned font/game archives and the correct Visual C++ runtime files for all three Windows architectures; see [package_windows_with_game.py](tools/package_windows_with_game.py) and [build_windows.ps1](tools/build_windows.ps1). The complete workflow prepares game archives and font caches during its Android stage before Windows packaging. A successful compilation alone does not replace package verification or a device test.

</details>

<a id="status"></a>

## Current limits and validation

Windows x64/x86 and Android ARM64 have local runtime evidence. Android ARM32 has device evidence from an earlier batch, with further full-flow regression still pending. Windows ARM64 packages have build and static checks but no hardware runtime validation. Linux and other upstream platforms are outside the current delivery matrix.

<details>
<summary><strong>Remaining work</strong></summary>

Remaining work includes complete playthrough and ending validation, full script proofreading, English word wrapping and Chinese punctuation layout, the touch inventory button's `Inv` label, broader controller/vibration and Android version coverage, interrupted-download recovery testing, audio-warning investigation, and 16 KB page-size validation. Traditional Chinese is planned for later.

</details>

<a id="credits"></a>

## Afterword and acknowledgements

Cave Story is the reason this project exists. To **Daisuke Amaya (Pixel / 天谷大輔)** and **[Studio Pixel](https://studiopixel.jp/)**: thank you for creating a world whose characters, music, and small details still make us want to return. This work is a tribute to that game and to the care behind it.

To the **[doukutsu-rs team and contributors](docs/AUTHORS.md)**: thank you for making the engine open, portable, and practical to build upon. The work in this fork depends on yours. We hope these additions help more players enjoy what you have made possible.

Thanks also to Hydrowing and Aeon Genesis for the translations, the Cave Story Tribute Site for preserving resources and information, and [Fusion Pixel](https://github.com/TakWolf/fusion-pixel-font) and its contributors for the pixel fonts. The inherited credits for AppleHair, Daedliy, ggez, Clownacy, LunarLambda/organism, Zoroyoshi, and other contributors remain in [AUTHORS.md](docs/AUTHORS.md).

> The hope is modest: that someone can open the game on a device they already own, read it comfortably, and spend a little more time in Cave Story's world.

<a id="license"></a>

## License and attribution

The engine is distributed under the [MIT License](LICENSE), retaining upstream attribution. Cave Story game data, translations, fonts, and other third-party assets retain their own terms and notices; the engine license does not grant rights to those assets. Fusion Pixel fonts use the SIL Open Font License, and vendored Trainer notices are included in [TRAINER-LICENSES.txt](vendor/trainer/notices/TRAINER-LICENSES.txt). Resource availability on a download site is not, by itself, permission for redistribution.

CaveStory-rs is a community project, independently maintained from Studio Pixel and doukutsu-rs. Its distribution reminders do not change the MIT license or the terms of third-party works.

<p align="center"><sub><a href="res/readme/ARTWORK.md">Cover illustration: AI-generated · Gallery: actual game captures · Asset credits</a></sub></p>
