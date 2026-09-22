<p align="center">
  <img src="../res/readme/hero.png" alt="CaveStory-rs" width="100%">
</p>

<p align="center">
  <strong>重返洞窟，用你喜欢的方式。</strong><br>
  中英日三语 · 自主手柄方案 · Windows 与 Android
</p>

<p align="center">
  <a href="../README.md">English</a> · <a href="README_zh-CN.md">简体中文</a> · <a href="README_ja-JP.md">日本語</a>
</p>

<p align="center">
  <img src="../res/readme/rust.svg" alt="Rust" height="24">
  <img src="../res/readme/windows.svg" alt="Windows" height="24">
  <img src="../res/readme/android.svg" alt="Android" height="24">
  <a href="../LICENSE"><img src="../res/readme/license.svg" alt="Engine license: MIT" height="24"></a>
</p>

<p align="center">
  <a href="#features">项目亮点</a> · <a href="#screenshots">游戏画面</a> · <a href="#play">开始游玩</a> · <a href="#build">构建</a> · <a href="#credits">致谢</a>
</p>

CaveStory-rs 是基于 [doukutsu-rs](https://github.com/doukutsu-rs/doukutsu-rs) 的社区分支。doukutsu-rs 使用 Rust 重新实现了《洞窟物语》的游戏引擎，本项目在此基础上整合简体中文、English、日本語游戏资源，并改善手柄操作、Android 首次启动和存档管理。当前主要面向 Windows、Android，以及 2004 年发布的免费原版游戏。

<a id="features"></a>

## 让喜欢的游戏，更顺手一点

<table>
  <tr>
    <td width="50%" valign="top"><h3>🌐 从菜单到剧情，三语切换</h3><p>菜单、对白、物品、地图、字幕和图片文字一起切换。简体中文、English、日本語，一套安装即可使用。</p></td>
    <td width="50%" valign="top"><h3>🎮 我们自己的手柄体验</h3><p>Xbox／PSP 布局、自定义按键、自动分配、热插拔和增强振动，组成自主设计的手柄方案。</p></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><h3>📳 Shizuku 蓝牙手柄振动</h3><p>我们开发的 Android 输出路径，通过 Shizuku 授权，让支持的蓝牙手柄真正振动起来。</p></td>
    <td width="50%" valign="top"><h3>📱 打开游戏，就能出发</h3><p>带游戏 APK 离线准备资源，自动匹配系统语言，触屏按键随外设接入与移除调整显示。</p></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><h3>💾 存档放在你想要的地方</h3><p>Android 可选私有或公共存档目录。迁移先复制校验，再切换位置，并保留原件。</p></td>
    <td width="50%" valign="top"><h3>🧩 可选修改器接口</h3><p>内置供独立 CaveStory-rs Trainer 使用的本机接口，由你决定何时启用。</p></td>
  </tr>
</table>

### 让手柄多一点真实的反馈

**通过 Shizuku 实现蓝牙手柄振动。** 我们开发了 Android 蓝牙振动输出路径：普通应用权限下无法正常输出时，通过 Shizuku 授权的 Shell 服务向支持的蓝牙手柄发送振动指令。这是我们尤其自豪的一项原创成果（我们觉得，这应该是一次前无古人的尝试）。实现覆盖玩家与设备的准确匹配、输出限幅限时、授权与重连，以及游戏退到后台时停止振动。已有实际设备的实体振动确认，包括未 root 的小米 9；兼容性仍取决于手柄、连接方式和系统。这是可选功能，正常游玩不需要 Shizuku。

<details>
<summary><strong>展开查看功能细节</strong></summary>

- **菜单与游戏资源同步切换三种语言。** 选择简体中文、English 或日本語时，剧情、物品说明、地图名称、结局字幕及相关图片文字会随界面一起切换。通过独立资源目录、编码适配和中日文字体，让三种语言可以在同一套安装中使用。全剧情逐句校对仍待完成。
- **自主设计与实现的手柄适配和振动方案。** 本项目实现了自动分配与热插拔改进、Xbox／PSP 按键布局预设、可保留的自定义按键及增强游戏振动，同时保留键盘、触屏与不同玩家的设备分配。振动默认开启并使用增强效果，已有明确设置继续保留；Android 将游戏振动效果与系统／蓝牙输出方式分开，并提供试振。
- **触屏按键与外设联动。** Android 检测手柄接入后自动隐藏触屏按键；没有相关外部输入设备时恢复显示。可选的“10 秒无触摸自动隐藏”默认关闭，触摸即可恢复，持续长按不会触发隐藏。
- **更顺畅的 Android 首次启动。** 带游戏版首次启动自动离线准备资源；基础版缺少资源时才询问下载，取消后正常退出。资源准备及错误说明提供中英日界面，在游戏字体尚不可用时也能正常阅读。
- **跟随 Android 系统／应用语言。** 新安装在简体中文语言环境下使用简体中文，日语环境下使用日本語，其余暂用 English。游戏内手动选择会保留，之后变更系统／应用语言时会重新生效；游玩过程中收到的变更延后至返回标题菜单应用，以保护当前现场。
- **存档位置选择与迁移。** Android 可使用私有目录或选定的本地公共目录。公共目录只保存存档、回放和进度记录，资源与设备设置继续私有。高级选项中的迁移功能先复制校验、再切换位置，保留原件并明确处理冲突。重新点击桌面图标进入游戏时，也会保护已经运行的游戏现场。
- **可选的本机修改器接口。** Windows 和 Android 已内置供独立维护的 CaveStory-rs Trainer 使用的游戏接口。Windows 需主动启用，例如设置 `CAVESTORY_TRAINER=1`；Android 需用户通过同签名 Trainer 明确授权。默认不允许连接。接口源码随游戏仓库提供，游戏可独立构建，修改器应用另行维护。
- **统一打包与获取渠道提醒。** 五种架构各提供基础版和带游戏版，并记录架构核验、资源来源及 SHA256。Android 首次来源问答只在本地保存回答，原生游戏启动前检查发行签名；剧情中的项目链接仅在玩家确认后打开。这些措施用于增加简单重签改包的成本，不能证明下载网站，也不能阻止所有修改。

</details>

<a id="screenshots"></a>

## 看看实际游戏画面

以下为本项目验证过程中保存的 Android 实际截图，点击可查看原图。

<table>
  <tr>
    <td width="50%"><a href="../res/readme/languages.png"><img src="../res/readme/languages.png" alt="同一套安装，三种语言" width="100%"></a><br><sub>同一套安装，三种语言</sub></td>
    <td width="50%"><a href="../res/readme/japanese-dialogue.png"><img src="../res/readme/japanese-dialogue.png" alt="游戏中的日文对白" width="100%"></a><br><sub>游戏中的日文对白</sub></td>
  </tr>
  <tr>
    <td width="50%"><a href="../res/readme/android-gameplay.png"><img src="../res/readme/android-gameplay.png" alt="Android 触屏操作" width="100%"></a><br><sub>Android 触屏操作</sub></td>
    <td width="50%"><a href="../res/readme/controller-options.png"><img src="../res/readme/controller-options.png" alt="自主手柄与振动选项" width="100%"></a><br><sub>自主手柄与振动选项</sub></td>
  </tr>
</table>

<a id="about"></a>

## 为什么有这个项目

初衷很简单：希望能在今天常用的各种设备上，使用自己熟悉的语言，方便地玩上《洞窟物语》。坐在电脑前可以用键盘或手柄，拿起手机也能继续享受这款游戏。

doukutsu-rs 为这个愿望提供了很好的基础。本项目最初着手补齐完整简体中文体验中的缺口：除了菜单，剧情、物品说明、地图名称、字体和图片文字也需要一起适配。随后逐步加入语言资源切换、更方便的资源准备、手柄配置、振动和 Android 存档管理等改进。

<a id="comparison"></a>

## 和原版有什么不同

### 与 2004 年的 PC 原版相比

原版《洞窟物语》是天谷大辅以 Pixel 之名创作的游戏。CaveStory-rs 使用 doukutsu-rs 开发的 Rust 引擎运行其游戏数据，继承了上游对现代系统、显示选项、可配置输入及 Android 触屏操作的支持。

本项目仍围绕免费原版的冒险、像素画面、音乐与玩法展开，补充多语言资源管理和各平台的使用便利性。另外，我们在已核实的巴尔罗格初遇对白中加入了一小段获取渠道提醒，因此所附脚本含有明确的原文增补。原有剧情及战斗选项保留，应用补丁前会备份原始脚本。

当前适配与验证范围是免费版数据及其译版。上游支持其他游戏版本和平台，并不代表本分支已经验证 Cave Story+、商业重制版或上游的全部移植平台。

### 相对 doukutsu-rs 的改进

本分支整合了完整三语资源切换、自主手柄与振动方案、Shizuku 蓝牙振动、Android 首启与存档管理，以及可选 Trainer 接口。上方的[项目亮点](#features)集中介绍这些新增成果，底层 Rust 引擎来自 doukutsu-rs。

<a id="play"></a>

## 开始游玩

当前本地交付矩阵如下：

| 平台 | 架构 | 包格式 |
| --- | --- | --- |
| Windows | `x86_64` · `x86_32` · `arm64` | ZIP，基础版／`_game` |
| Android | `arm64-v8a` · `armeabi-v7a` | APK，基础版／`_game` |

- **带游戏版（`_game`）：**包含三语资源和字体，Windows 另附对应运行库；Android 可离线准备内置资源。
- **基础版：**Windows 需配备兼容游戏数据和对应 Visual C++ 运行库；Android 缺资源时提供下载。

文件名为 `CaveStory-rs_<平台>_<YYYYMMDD>_<架构>[_game].zip` 或 `.apk`。Android 为两个独立 ABI 包，请按设备系统实际支持的架构选择。当前配置的最低 Android 版本为 Android 7.0／API 24，不代表所有符合版本要求的设备都已实测。

本项目从 [GitHub Releases](https://github.com/TG-Twilight/CaveStory-rs/releases/latest) 下载。当前发行包在本地构建并核验；旧上游工作流已停用，本 fork 的云端编译和自动发布流程仍待适配验证。

### Windows

将带游戏版完整解压到可写目录，启动 `CaveStory-rs.exe`。初始语言为简体中文，可在**选项 → 语言**中选择 English 或日本語。升级或搬迁时保留 `user/`，其中保存了存档和设置；原始 `Doukutsu.exe` 及语言目录中的原程序用于提取资源，也请保留。

<details>
<summary><strong>基础版安装与补齐语言资源</strong></summary>

基础版需要配合兼容免费版数据的工作副本使用。已验证的中文安装方式是将所附 `data/fonts`、`data/locale` 合入简体中文游戏副本，并把 `CaveStory-rs.exe` 放在 `Doukutsu.exe` 旁。保留已有存档和自定义文件。

可识别的受管理中文安装可以用以下命令补齐英文或日文资源，路径需替换为实际 `data` 目录。工具会保护已存在的语言目录，不直接覆盖：

```powershell
py tools/install_english_resources.py --data "C:/Games/CaveStory-rs/data"
py tools/install_japanese_resources.py --data "C:/Games/CaveStory-rs/data"
```

</details>

### Android

安装对应 ABI 的 APK，完成资源准备后选择私有或公共存档目录。公共目录支持系统选择器提供的本地文件夹，暂不支持云盘和任意第三方文件提供器。私有文件仍可通过文件管理器的**添加存储**入口访问游戏的文档提供器。

之后如需移动存档，返回标题菜单，打开**选项 → 高级选项 → 存档位置**。迁移会保留原文件；跨安装或跨设备移动时，也请另行保留备份。

基础版与带游戏正式版共用 `io.github.cavestory_rs` 和同一签名，可以相互覆盖更新。旧 `io.github.doukutsu_rs` 应用及 debug 测试版使用独立存储，不会自动导入其存档。

### 游戏资源来源

资源入口为 [Cave Story Tribute Site](https://www.cavestory.one/download/cave-story.php)。简体中文译版署名 Hydrowing，英文译版署名 Aeon Genesis，日语原包来自 Studio Pixel。打包工具记录所用压缩包及哈希。带游戏包用于简化安装，减少玩家对纪念站的重复下载；完整原始游戏数据不放入源码仓库或基础包。

仓库目录和贡献规则：[贡献指南](CONTRIBUTING.md)。

<a id="build"></a>

## 如何构建

按需展开对应构建方式。所有产物统一放在仓库旁的 CaveStory-rs-runs 目录。

<details>
<summary><strong>准备源码与工具</strong></summary>

```sh
git clone --recurse-submodules https://github.com/TG-Twilight/CaveStory-rs.git
cd CaveStory-rs
git submodule update --init --recursive
```

需要 Rust／Cargo、Git、C/C++ 编译工具链和 CMake。[Cargo.toml](../Cargo.toml) 声明的最低 Rust 版本为 1.88；已有本地构建记录使用 1.98.1，最低版本本身尚未验证。默认桌面后端会从源码构建 SDL2。资源准备还需要 Python 3 和 Pillow（Windows 可运行 `py -m pip install Pillow`）。

构建输出统一放在仓库旁的 `CaveStory-rs-runs/`。[.cargo/config.toml](../.cargo/config.toml) 已将 Cargo 输出指向该目录，下载和打包缓存也位于源码树之外。游戏侧 Trainer 依赖已包含在 `vendor/trainer/` 中，无需另行检出修改器仓库。

</details>

<details>
<summary><strong>Windows 引擎编译</strong></summary>

安装 Visual Studio C++ Build Tools 和 Windows SDK，在已配置目标架构的开发者命令环境中运行，并确保 `PATH` 包含 CMake。以下编译 x64 引擎：

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release --locked --bin CaveStory-rs --target x86_64-pc-windows-msvc
```

程序位于 `../CaveStory-rs-runs/cache/cargo/x86_64-pc-windows-msvc/release/CaveStory-rs.exe`。此命令只编译引擎，运行时按前文准备资源。另两种 Windows target 为 `i686-pc-windows-msvc` 和 `aarch64-pc-windows-msvc`，分别需要对应的 MSVC 工具及库。

</details>

<details>
<summary><strong>Android 编译</strong></summary>

当前工程使用兼容 JDK 17 的工具环境、SDK 35、Build Tools `35.0.1`、NDK `28.0.13004108` 及 CMake 3.22 或更新版本。Gradle wrapper 与 Android Gradle Plugin 的版本由仓库配置指定。先安装两个 Rust target：

```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi
```

**构建前必须配置签名。** 当前 Debug 和 Release 均要求 JKS，不会自动回退到默认 debug 密钥。Gradle 接受 `REVIA_KS_PATH`、`REVIA_KS_PASS`，要求密钥库中只有一个私钥条目，并使用相同的 Store／Key password。维护者私钥不随源码提供。自行构建时，使用自己的密钥，并将 [SignaturePolicy.java](../platforms/android/app/src/main/java/io/github/cavestory_rs/SignaturePolicy.java) 配置为自己的证书，否则安装后会被启动检查拒绝。再分发时请明确标明是自己的构建。

设置好签名环境变量和 `JAVA_HOME`／`ANDROID_HOME` 后，在仓库根目录运行：

```powershell
cd platforms/android
./gradlew.bat assembleDebug --project-cache-dir ../../../CaveStory-rs-runs/cache/gradle-project --console=plain
cd ../..
```

两个基础 debug APK 输出到 `../CaveStory-rs-runs/cache/android/app/build/outputs/apk/debug/`，应用标识带 `.debug` 后缀。使用 `assembleRelease` 构建正式变体。需要内置游戏资源时，先运行 `py tools/package_windows_with_game.py --android-assets <CaveStory-rs-runs内的目录>` 准备核验过的资源，再向 Gradle 传入 `-PcaveStoryGameAssets=<该目录>`。缓存为空时，首次依赖、字体及游戏资源准备需要联网。

</details>

<details>
<summary><strong>本地完整交付：五架构、十份成品</strong></summary>

维护者使用以下入口：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build_all.ps1
```

脚本在 `../CaveStory-rs-runs/builds/<批次>/` 中生成 Android 两种 ARM APK 和 Windows 三种架构 ZIP，每种都有基础版与带游戏版，同批使用统一日期版本。批次附带 `build-info.json`、`SHA256SUMS` 和 `verified-delivery.json`。

这套流程目前面向维护者的 Windows 环境。在其他机器上使用前，需要调整 [build_windows.cmd](../tools/build_windows.cmd)、[cargo_windows_arm64.ps1](../tools/cargo_windows_arm64.ps1)、[windows-arm64.cmake](../tools/windows-arm64.cmake) 中的 Visual Studio 2019／MSVC 和 ARM64 工具路径，以及 [build_android.ps1](../tools/build_android.ps1) 中的 JBR／SDK 路径。Android 脚本从 Windows **用户级**环境变量 `REVIA_KS_PASS` 读取密码；APK 校验脚本还固定检查维护者证书指纹，自行发行需要与应用签名策略一起调整。

同时需准备固定版本的字体／游戏压缩包，以及三种 Windows 架构对应的 Visual C++ 运行库，具体见 [package_windows_with_game.py](../tools/package_windows_with_game.py) 和 [build_windows.ps1](../tools/build_windows.ps1)。完整流程先在 Android 阶段准备游戏压缩包和字体缓存，再进行 Windows 打包。编译成功之后仍需检查包内容及实际运行情况。

</details>

<a id="status"></a>

## 当前限制与验证范围

Windows x64／x86 和 Android ARM64 已有本地运行证据。Android ARM32 已有较早批次的实机记录，完整流程回归仍待补齐；Windows ARM64 已完成构建与静态核验，尚无硬件运行验收。Linux 及其他上游平台暂不在本项目的交付矩阵中。

<details>
<summary><strong>展开查看剩余事项</strong></summary>

后续事项包括完整通关与结局验证、全剧情校对、英文单词换行和中文标点排版、触屏物品按钮的 `Inv` 文字、更多手柄／振动与 Android 版本覆盖、下载中断恢复测试、音频警告调查及 16 KB 页验证。繁体中文留待后续。

</details>

<a id="credits"></a>

## 后记与致谢

因为《洞窟物语》，才有了这个项目。向 **天谷大辅（天谷大輔／Daisuke Amaya／Pixel）**与 **[Studio Pixel](https://studiopixel.jp/)** 致敬。感谢你创造了这个世界，让其中的人物、音乐和小小细节，在多年后仍使人愿意再次走进洞窟。这份工作，也是一份献给原作及其创作心意的敬意。

感谢 **[doukutsu-rs 团队和每一位贡献者](AUTHORS.md)**。你们让引擎保持开放，让这款游戏走上更多平台，也让后来的开发者能够继续改进。本项目的工作建立在你们的成果之上，希望这些补充能让更多玩家享受到你们所带来的可能。

也感谢 Hydrowing、Aeon Genesis 的翻译，Cave Story Tribute Site 对资源和资料的保存，以及 [Fusion Pixel](https://github.com/TakWolf/fusion-pixel-font) 和贡献者提供的像素字体。沿用自上游的 AppleHair、Daedliy、ggez、Clownacy、LunarLambda／organism、Zoroyoshi 等贡献者署名保留在 [AUTHORS.md](AUTHORS.md)。

> 愿这份小小的改进，让某位玩家能在手边的设备上打开游戏，读懂对白，再在《洞窟物语》的世界里多停留一会儿。

<a id="license"></a>

## 许可与署名

引擎以 [MIT 许可证](../LICENSE) 分发并保留上游署名。游戏数据、译文、字体及其他第三方素材各自遵循原有条款和说明，引擎许可不代表取得这些素材的授权。Fusion Pixel 字体使用 SIL Open Font License；随附的 Trainer 许可见 [TRAINER-LICENSES.txt](../vendor/trainer/notices/TRAINER-LICENSES.txt)。资源可从网站下载，本身不等于取得再分发许可。

CaveStory-rs 是独立于 Studio Pixel 和 doukutsu-rs 维护的社区项目。获取渠道提醒不改变 MIT 许可证或第三方作品的许可条件。

<p align="center"><sub><a href="../res/readme/ARTWORK.md">封面：AI 生成插画 · 展示图：实际游戏截图 · 图片来源与署名</a></sub></p>
