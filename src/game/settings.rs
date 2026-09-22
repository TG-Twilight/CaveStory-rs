use crate::framework::context::Context;
use crate::framework::error::GameResult;
use crate::framework::filesystem::{user_create, user_open};
use crate::framework::gamepad::{Axis, AxisDirection, Button, GamepadContext, PlayerControllerInputType};
use crate::framework::graphics::VSyncMode;
use crate::framework::keyboard::ScanCode;
use crate::game::player::TargetPlayer;
use crate::game::shared_game_state::{CutsceneSkipMode, ScreenShakeIntensity, TimingMode, WindowMode};
use crate::input::combined_player_controller::CombinedPlayerController;
use crate::input::gamepad_player_controller::GamepadController;
use crate::input::keyboard_player_controller::KeyboardController;
use crate::input::player_controller::PlayerController;
use crate::input::touch_player_controller::TouchPlayerController;
use crate::sound::InterpolationMode;

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RumbleMode {
    Original,
    Enhanced,
}

impl Default for RumbleMode {
    fn default() -> Self {
        Self::Enhanced
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default = "default_true")]
    pub seasonal_textures: bool,
    pub original_textures: bool,
    pub shader_effects: bool,
    #[serde(default = "default_true")]
    pub light_cone: bool,
    #[serde(default = "default_true")]
    pub subpixel_coords: bool,
    #[serde(default = "default_true")]
    pub motion_interpolation: bool,
    pub touch_controls: bool,
    #[serde(default = "default_true")]
    pub display_touch_controls: bool,
    #[serde(default)]
    pub auto_hide_touch_controls: bool,
    pub soundtrack: String,
    #[serde(default = "default_vol")]
    pub bgm_volume: f32,
    #[serde(default = "default_vol")]
    pub sfx_volume: f32,
    #[serde(default = "default_timing")]
    pub timing_mode: TimingMode,
    #[serde(default = "default_pause_on_focus_loss")]
    pub pause_on_focus_loss: bool,
    #[serde(default = "default_interpolation")]
    pub organya_interpolation: InterpolationMode,
    #[serde(default = "default_p1_controller_type")]
    pub player1_controller_type: ControllerType,
    #[serde(default = "default_p2_controller_type")]
    pub player2_controller_type: ControllerType,
    #[serde(default = "default_true")]
    pub auto_controller_switching: bool,
    #[serde(default = "p1_default_keymap")]
    pub player1_key_map: PlayerKeyMap,
    #[serde(default = "p2_default_keymap")]
    pub player2_key_map: PlayerKeyMap,
    #[serde(default = "player_default_controller_button_map")]
    pub player1_controller_button_map: PlayerControllerButtonMap,
    #[serde(default = "player_default_controller_button_map")]
    pub player2_controller_button_map: PlayerControllerButtonMap,
    #[serde(default)]
    pub player1_custom_controller_button_map: Option<PlayerControllerButtonMap>,
    #[serde(default)]
    pub player2_custom_controller_button_map: Option<PlayerControllerButtonMap>,
    #[serde(default = "default_controller_axis_sensitivity")]
    pub player1_controller_axis_sensitivity: f64,
    #[serde(default = "default_controller_axis_sensitivity")]
    pub player2_controller_axis_sensitivity: f64,
    #[serde(default = "default_rumble")]
    pub player1_rumble: bool,
    #[serde(default = "default_rumble")]
    pub player2_rumble: bool,
    #[serde(default)]
    pub player1_rumble_mode: RumbleMode,
    #[serde(default)]
    pub player2_rumble_mode: RumbleMode,
    #[serde(skip, default = "default_speed")]
    pub speed: f64,
    #[serde(skip)]
    pub god_mode: bool,
    #[serde(skip)]
    pub infinite_booster: bool,
    #[serde(skip)]
    pub debug_outlines: bool,
    pub fps_counter: bool,
    pub locale: String,
    // Old settings without these fields represent an explicit saved language.
    #[serde(default)]
    pub follow_system_language: bool,
    #[serde(default)]
    pub android_language: Option<String>,
    #[serde(default = "default_window_mode")]
    pub window_mode: WindowMode,
    #[serde(default = "default_vsync")]
    pub vsync_mode: VSyncMode,
    #[serde(default = "default_screen_shake_intensity")]
    pub screen_shake_intensity: ScreenShakeIntensity,
    pub debug_mode: bool,
    #[serde(skip)]
    pub noclip: bool,
    pub more_rust: bool,
    #[serde(default = "default_cutscene_skip_mode")]
    pub cutscene_skip_mode: CutsceneSkipMode,
    #[serde(default = "default_true")]
    pub discord_rpc: bool,
    #[serde(default = "default_true")]
    pub allow_strafe: bool,
}

fn default_true() -> bool {
    true
}

#[inline(always)]
fn current_version() -> u32 {
    26
}

#[inline(always)]
fn default_timing() -> TimingMode {
    TimingMode::_50Hz
}

#[inline(always)]
fn default_window_mode() -> WindowMode {
    WindowMode::Windowed
}

#[inline(always)]
fn default_interpolation() -> InterpolationMode {
    InterpolationMode::Linear
}

#[inline(always)]
fn default_speed() -> f64 {
    1.0
}

#[inline(always)]
fn default_vol() -> f32 {
    1.0
}

#[inline(always)]
fn default_locale() -> String {
    "en".to_string()
}

#[inline(always)]
fn default_vsync() -> VSyncMode {
    VSyncMode::VSync
}

#[inline(always)]
fn default_screen_shake_intensity() -> ScreenShakeIntensity {
    ScreenShakeIntensity::Full
}

#[inline(always)]
fn default_p1_controller_type() -> ControllerType {
    if cfg!(any(target_os = "horizon")) {
        ControllerType::Gamepad(0)
    } else {
        ControllerType::Keyboard
    }
}

#[inline(always)]
fn default_p2_controller_type() -> ControllerType {
    if cfg!(any(target_os = "horizon")) {
        ControllerType::Gamepad(1)
    } else {
        ControllerType::Keyboard
    }
}

#[inline(always)]
fn default_pause_on_focus_loss() -> bool {
    true
}

#[inline(always)]
fn default_rumble() -> bool {
    true
}

#[inline(always)]
fn default_cutscene_skip_mode() -> CutsceneSkipMode {
    CutsceneSkipMode::Hold
}

impl Settings {
    pub fn load(ctx: &Context) -> GameResult<Settings> {
        if let Ok(file) = user_open(ctx, "/settings.json") {
            match serde_json::from_reader::<_, Settings>(file) {
                Ok(settings) => {
                    #[allow(unused_mut)]
                    let mut settings = settings.upgrade();
                    #[cfg(target_os = "android")]
                    settings.load_android_language(ctx);
                    return Ok(settings);
                }
                Err(err) => log::warn!("Failed to deserialize settings: {}", err),
            }
        }

        let mut settings = Settings::default();
        // A marker is published with the complete Chinese data tree. Preserve all
        // saved choices, including when an existing settings file is unreadable.
        #[cfg(not(target_os = "android"))]
        if !crate::framework::filesystem::user_exists(ctx, "/settings.json")
            && crate::framework::filesystem::exists(ctx, "/chinese-install.json")
            && crate::framework::filesystem::exists(ctx, "/locale/zh-Hans.json")
        {
            settings.locale = "zh-Hans".to_owned();
        }
        #[cfg(target_os = "android")]
        if !crate::framework::filesystem::user_exists(ctx, "/settings.json") {
            settings.load_android_language(ctx);
        }
        Ok(settings)
    }

    #[cfg(any(target_os = "android", test))]
    fn apply_android_language(&mut self, selection: &str) {
        let mut parts = selection.splitn(3, ':');
        let (Some(source), Some(locale)) = (parts.next(), parts.next()) else { return; };
        if !matches!(source, "app" | "system") || !matches!(locale, "en" | "jp" | "zh-Hans") { return; }
        let changed = self.android_language.as_deref().map_or(source == "app", |previous| previous != selection);
        if self.follow_system_language || changed {
            self.locale = locale.into();
            self.follow_system_language = true;
        }
        self.android_language = Some(selection.into());
    }

    #[cfg(target_os = "android")]
    pub fn load_android_language(&mut self, ctx: &Context) -> bool {
        let previous_locale = self.locale.clone();
        match crate::framework::android_locale::language() {
            Ok(selection) => {
                let previous = self.android_language.clone();
                if previous.as_deref() != Some(&selection) {
                    // Do not replace a user's unreadable settings during automatic
                    // locale refresh, including the first title tick after load.
                    if let Ok(file) = user_open(ctx, "/settings.json") {
                        if serde_json::from_reader::<_, Settings>(file).is_err() { return false; }
                    }
                }
                self.apply_android_language(&selection);
                // Persist the platform baseline so unchanged defaults do not erase a manual choice.
                if previous != self.android_language {
                    if let Err(error) = self.save(ctx) { log::warn!("Cannot save Android language: {}", error); }
                    log::info!("Android language: {}, game: {}, automatic: {}", selection, self.locale, self.follow_system_language);
                }
            }
            Err(error) => log::warn!("Cannot read Android language: {}", error),
        }
        previous_locale != self.locale
    }

    fn upgrade(mut self) -> Self {
        // Android must always retain a touch input path, including imported settings.
        #[cfg(target_os = "android")]
        { self.touch_controls = true; }
        let initial_version = self.version;

        if self.version == 2 {
            self.version = 3;
            self.light_cone = true;
        }

        if self.version == 3 {
            self.version = 4;
            self.timing_mode = default_timing();
        }

        if self.version == 4 {
            self.version = 5;
            self.bgm_volume = default_vol();
            self.sfx_volume = default_vol();
        }

        if self.version == 5 {
            self.version = 6;
            self.player1_key_map.strafe = ScanCode::LShift;
            self.player2_key_map.strafe = ScanCode::RShift;
        }

        if self.version == 6 {
            self.version = 7;
            self.locale = default_locale();
        }

        if self.version == 7 {
            self.version = 8;
            self.vsync_mode = default_vsync();
        }

        if self.version == 8 {
            self.version = 9;
            self.debug_mode = false;
        }

        if self.version == 9 {
            self.version = 10;
            self.screen_shake_intensity = default_screen_shake_intensity();
        }

        if self.version == 10 {
            self.version = 11;
            self.window_mode = default_window_mode();
        }

        if self.version == 11 {
            self.version = 12;
            self.player1_controller_type = default_p1_controller_type();
            self.player2_controller_type = default_p2_controller_type();
            self.player1_controller_button_map = player_default_controller_button_map();
            self.player2_controller_button_map = player_default_controller_button_map();
            self.player1_controller_axis_sensitivity = default_controller_axis_sensitivity();
            self.player2_controller_axis_sensitivity = default_controller_axis_sensitivity();
        }

        if self.version == 12 {
            self.version = 13;

            if self.player1_key_map.skip == ScanCode::E {
                self.player1_key_map.skip = ScanCode::Q;
            }

            if self.player2_key_map.skip == ScanCode::U {
                self.player2_key_map.skip = ScanCode::T;
            }

            // reset controller mappings since we've updated enums
            self.player1_controller_button_map = player_default_controller_button_map();
            self.player2_controller_button_map = player_default_controller_button_map();
        }

        if self.version == 13 {
            self.version = 14;

            // reset controller mappings again since we have new enums
            self.player1_controller_button_map = player_default_controller_button_map();
            self.player2_controller_button_map = player_default_controller_button_map();
        }

        if self.version == 14 {
            self.version = 15;
            self.pause_on_focus_loss = default_pause_on_focus_loss();
        }

        if self.version == 15 {
            self.version = 16;

            self.player1_key_map.menu_ok = self.player1_key_map.jump;
            self.player1_key_map.menu_back = self.player1_key_map.shoot;
            self.player1_controller_button_map.menu_ok = self.player1_controller_button_map.jump;
            self.player1_controller_button_map.menu_back = self.player1_controller_button_map.shoot;

            self.player2_key_map.menu_ok = self.player2_key_map.jump;
            self.player2_key_map.menu_back = self.player2_key_map.shoot;
            self.player2_controller_button_map.menu_ok = self.player2_controller_button_map.jump;
            self.player2_controller_button_map.menu_back = self.player2_controller_button_map.shoot;
        }

        if self.version == 16 {
            self.version = 17;

            if self.player1_controller_button_map.shoot == PlayerControllerInputType::ButtonInput(Button::East) {
                self.player1_controller_button_map.shoot = PlayerControllerInputType::ButtonInput(Button::West);
            }

            if self.player2_controller_button_map.shoot == PlayerControllerInputType::ButtonInput(Button::East) {
                self.player2_controller_button_map.shoot = PlayerControllerInputType::ButtonInput(Button::West);
            }

            if self.player1_controller_button_map.map == PlayerControllerInputType::ButtonInput(Button::West) {
                self.player1_controller_button_map.map = PlayerControllerInputType::ButtonInput(Button::East);
            }

            if self.player2_controller_button_map.map == PlayerControllerInputType::ButtonInput(Button::West) {
                self.player2_controller_button_map.map = PlayerControllerInputType::ButtonInput(Button::East);
            }
        }

        if self.version == 17 {
            self.version = 18;
            self.player1_rumble = default_rumble();
            self.player2_rumble = default_rumble();
        }

        if self.version == 18 {
            self.version = 19;
            self.more_rust = false;
        }

        if self.version == 19 {
            self.version = 20;
            self.cutscene_skip_mode = CutsceneSkipMode::Hold;
        }

        if self.version == 20 {
            self.version = 21;

            self.locale = match self.locale.as_str() {
                "English" => "en".to_string(),
                "Japanese" => "jp".to_string(),
                _ => default_locale(),
            };
        }

        if self.version == 21 {
            self.version = 22;
            self.discord_rpc = true;
        }

        if self.version == 22 {
            self.version = 23;
            self.display_touch_controls = true;
        }

        if self.version == 23 {
            self.version = 24;
            self.allow_strafe = true;
        }

        if self.version == 24 {
            self.version = 25;
            self.soundtrack = match self.soundtrack.as_str() {
                "Organya" => "organya".to_owned(),
                "Remastered" => "remastered".to_owned(),
                "New" => "new".to_owned(),
                "Famitracks" => "famitracks".to_owned(),
                "Ridiculon" => "ridiculon".to_owned(),
                _ => self.soundtrack.clone(),
            }
        }

        if self.version == 25 {
            self.version = 26;
        }

        if self.version != initial_version {
            log::info!("Upgraded configuration file from version {} to {}.", initial_version, self.version);
        }

        self
    }

    pub fn save(&self, ctx: &Context) -> GameResult {
        let file = user_create(ctx, "/settings.json")?;
        serde_json::to_writer_pretty(file, self)?;

        Ok(())
    }

    pub fn create_player1_controller(&self) -> Box<dyn PlayerController> {
        let mut combined = CombinedPlayerController::new();
        combined.add(Box::new(KeyboardController::new(TargetPlayer::Player1)));
        // Keep it alive even in keyboard mode so hotplug and settings changes
        // work in an already-running scene, including after closing pause menus.
        combined.add(Box::new(GamepadController::new(0, TargetPlayer::Player1)));
        if self.touch_controls {
            combined.add(Box::new(TouchPlayerController::new()));
        }
        Box::new(combined)
    }

    pub(crate) fn player1_gamepad_index(&self, pads: &GamepadContext) -> Option<u32> {
        if self.auto_controller_switching {
            let excluded = match self.player2_controller_type {
                ControllerType::Gamepad(index) => Some(index),
                ControllerType::Keyboard => None,
            };
            pads.automatic_gamepad_index(excluded)
        } else if let ControllerType::Gamepad(index) = self.player1_controller_type {
            pads.instance_id(index).map(|_| index)
        } else {
            None
        }
    }

    pub fn create_player2_controller(&self) -> Box<dyn PlayerController> {
        match self.player2_controller_type {
            ControllerType::Keyboard => Box::new(KeyboardController::new(TargetPlayer::Player2)),
            ControllerType::Gamepad(index) => {
                let keyboard_controller = Box::new(KeyboardController::new(TargetPlayer::Player2));

                let mut gamepad_controller = Box::new(GamepadController::new(index, TargetPlayer::Player2));
                gamepad_controller.set_rumble_enabled(self.player2_rumble);

                let mut combined_player_controller = CombinedPlayerController::new();
                combined_player_controller.add(keyboard_controller);
                combined_player_controller.add(gamepad_controller);

                Box::new(combined_player_controller)
            }
        }
    }

    pub fn get_gamepad_axis_sensitivity(&self, id: u32) -> f64 {
        if self.player2_controller_type == ControllerType::Gamepad(id) {
            self.player2_controller_axis_sensitivity
        } else if self.auto_controller_switching || self.player1_controller_type == ControllerType::Gamepad(id) {
            self.player1_controller_axis_sensitivity
        } else {
            default_controller_axis_sensitivity()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: current_version(),
            seasonal_textures: true,
            original_textures: false,
            shader_effects: false,
            light_cone: true,
            subpixel_coords: true,
            motion_interpolation: true,
            touch_controls: cfg!(target_os = "android"),
            display_touch_controls: true,
            auto_hide_touch_controls: false,
            soundtrack: "Organya".to_string(),
            bgm_volume: 1.0,
            sfx_volume: 1.0,
            timing_mode: default_timing(),
            pause_on_focus_loss: default_pause_on_focus_loss(),
            organya_interpolation: InterpolationMode::Linear,
            player1_controller_type: default_p1_controller_type(),
            player2_controller_type: default_p2_controller_type(),
            auto_controller_switching: true,
            player1_key_map: p1_default_keymap(),
            player2_key_map: p2_default_keymap(),
            player1_controller_button_map: player_default_controller_button_map(),
            player2_controller_button_map: player_default_controller_button_map(),
            player1_custom_controller_button_map: None,
            player2_custom_controller_button_map: None,
            player1_controller_axis_sensitivity: default_controller_axis_sensitivity(),
            player2_controller_axis_sensitivity: default_controller_axis_sensitivity(),
            player1_rumble: default_rumble(),
            player2_rumble: default_rumble(),
            player1_rumble_mode: RumbleMode::default(),
            player2_rumble_mode: RumbleMode::default(),
            speed: 1.0,
            god_mode: false,
            infinite_booster: false,
            debug_outlines: false,
            fps_counter: false,
            locale: default_locale(),
            follow_system_language: cfg!(target_os = "android"),
            android_language: None,
            window_mode: WindowMode::Windowed,
            vsync_mode: VSyncMode::VSync,
            screen_shake_intensity: ScreenShakeIntensity::Full,
            debug_mode: false,
            noclip: false,
            more_rust: false,
            cutscene_skip_mode: CutsceneSkipMode::Hold,
            discord_rpc: true,
            allow_strafe: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlayerKeyMap {
    pub left: ScanCode,
    pub up: ScanCode,
    pub right: ScanCode,
    pub down: ScanCode,
    pub prev_weapon: ScanCode,
    pub next_weapon: ScanCode,
    pub jump: ScanCode,
    pub shoot: ScanCode,
    pub skip: ScanCode,
    pub inventory: ScanCode,
    pub map: ScanCode,
    pub strafe: ScanCode,
    pub menu_ok: ScanCode,
    pub menu_back: ScanCode,
}

#[inline(always)]
pub fn p1_default_keymap() -> PlayerKeyMap {
    PlayerKeyMap {
        left: ScanCode::Left,
        up: ScanCode::Up,
        right: ScanCode::Right,
        down: ScanCode::Down,
        prev_weapon: ScanCode::A,
        next_weapon: ScanCode::S,
        jump: ScanCode::Z,
        shoot: ScanCode::X,
        skip: ScanCode::Q,
        inventory: ScanCode::Q,
        map: ScanCode::W,
        strafe: ScanCode::LShift,
        menu_ok: ScanCode::Z,
        menu_back: ScanCode::X,
    }
}

#[inline(always)]
pub fn p2_default_keymap() -> PlayerKeyMap {
    PlayerKeyMap {
        left: ScanCode::Comma,
        up: ScanCode::L,
        right: ScanCode::Slash,
        down: ScanCode::Period,
        prev_weapon: ScanCode::G,
        next_weapon: ScanCode::H,
        jump: ScanCode::B,
        shoot: ScanCode::N,
        skip: ScanCode::T,
        inventory: ScanCode::T,
        map: ScanCode::Y,
        strafe: ScanCode::RShift,
        menu_ok: ScanCode::B,
        menu_back: ScanCode::N,
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Copy, Clone)]
pub enum ControllerType {
    Keyboard,
    Gamepad(u32),
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PlayerControllerButtonMap {
    pub left: PlayerControllerInputType,
    pub up: PlayerControllerInputType,
    pub right: PlayerControllerInputType,
    pub down: PlayerControllerInputType,
    pub prev_weapon: PlayerControllerInputType,
    pub next_weapon: PlayerControllerInputType,
    pub jump: PlayerControllerInputType,
    pub shoot: PlayerControllerInputType,
    pub skip: PlayerControllerInputType,
    pub inventory: PlayerControllerInputType,
    pub map: PlayerControllerInputType,
    pub strafe: PlayerControllerInputType,
    pub menu_ok: PlayerControllerInputType,
    pub menu_back: PlayerControllerInputType,
}

#[inline(always)]
pub fn player_default_controller_button_map() -> PlayerControllerButtonMap {
    let mut map = legacy_controller_button_map();
    map.map = PlayerControllerInputType::EitherButtons(Button::North, Button::Back);
    map
}

impl PlayerControllerButtonMap {
    pub fn layout_index(&self) -> usize {
        if *self == player_default_controller_button_map() || *self == legacy_controller_button_map() {
            0
        } else if *self == psp_controller_button_map() {
            1
        } else {
            2
        }
    }

    pub fn switch_layout(&mut self, layout: usize, custom: &mut Option<Self>) {
        if self.layout_index() == 2 {
            *custom = Some(self.clone());
        }
        match layout {
            0 => *self = player_default_controller_button_map(),
            1 => *self = psp_controller_button_map(),
            2 => {
                if let Some(saved) = custom {
                    *self = saved.clone();
                }
            }
            _ => {}
        }
    }
}

pub fn psp_controller_button_map() -> PlayerControllerButtonMap {
    let mut map = player_default_controller_button_map();
    map.menu_ok = PlayerControllerInputType::ButtonInput(Button::East);
    map.menu_back = PlayerControllerInputType::ButtonInput(Button::South);
    map
}

fn legacy_controller_button_map() -> PlayerControllerButtonMap {
    PlayerControllerButtonMap {
        left: PlayerControllerInputType::Either(Button::DPadLeft, Axis::LeftX, AxisDirection::Left),
        up: PlayerControllerInputType::Either(Button::DPadUp, Axis::LeftY, AxisDirection::Up),
        right: PlayerControllerInputType::Either(Button::DPadRight, Axis::LeftX, AxisDirection::Right),
        down: PlayerControllerInputType::Either(Button::DPadDown, Axis::LeftY, AxisDirection::Down),
        prev_weapon: PlayerControllerInputType::ButtonInput(Button::LeftShoulder),
        next_weapon: PlayerControllerInputType::ButtonInput(Button::RightShoulder),
        jump: PlayerControllerInputType::ButtonInput(Button::East),
        shoot: PlayerControllerInputType::ButtonInput(Button::South),
        skip: PlayerControllerInputType::ButtonInput(Button::West),
        strafe: PlayerControllerInputType::AxisInput(Axis::TriggerRight, AxisDirection::Either),
        inventory: PlayerControllerInputType::ButtonInput(Button::West),
        map: PlayerControllerInputType::ButtonInput(Button::North),
        menu_ok: PlayerControllerInputType::ButtonInput(Button::South),
        menu_back: PlayerControllerInputType::ButtonInput(Button::East),
    }
}

#[inline(always)]
pub fn default_controller_axis_sensitivity() -> f64 {
    0.3
}

#[cfg(test)]
mod controller_mapping_tests {
    use super::*;
    use crate::game::shared_game_state::SharedGameState;

    #[test]
    fn chinese_install_defaults_only_without_saved_settings() {
        use crate::framework::vfs::PhysicalFS;
        let root = std::env::temp_dir().join(format!("cavestory-settings-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(root.join("data/locale")).unwrap();
        std::fs::create_dir(root.join("saves")).unwrap();
        let mut ctx = Context::new();
        ctx.filesystem.mount_vfs(Box::new(PhysicalFS::new(&root.join("data"), true)));
        ctx.filesystem.mount_user_vfs(Box::new(PhysicalFS::new(&root.join("saves"), false)));
        assert_eq!(Settings::load(&ctx).unwrap().locale, "en");
        std::fs::write(root.join("data/chinese-install.json"), b"{}").unwrap();
        assert_eq!(Settings::load(&ctx).unwrap().locale, "en", "incomplete install");
        std::fs::write(root.join("data/locale/zh-Hans.json"), b"{}").unwrap();
        assert_eq!(Settings::load(&ctx).unwrap().locale, "zh-Hans");
        let mut settings = Settings::default();
        settings.locale = "en".to_owned();
        settings.player1_rumble = false;
        settings.save(&ctx).unwrap();
        let saved = std::fs::read(root.join("saves/settings.json")).unwrap();
        let loaded = Settings::load(&ctx).unwrap();
        assert_eq!(loaded.locale, "en");
        assert!(!loaded.player1_rumble);
        assert_eq!(std::fs::read(root.join("saves/settings.json")).unwrap(), saved);
        std::fs::write(root.join("saves/settings.json"), b"invalid").unwrap();
        assert_eq!(Settings::load(&ctx).unwrap().locale, "en", "do not reinterpret corrupt saved choices");
        drop(ctx);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn default_english_locale_still_selects_english_resource_directory() {
        let mut constants = crate::engine_constants::EngineConstants::defaults();
        constants.active_root.support_locales = true;
        let mut settings = Settings::default();
        constants.rebuild_path_list(None, crate::game::shared_game_state::Season::current(), &settings);
        assert_eq!(constants.base_paths[0], "/en/");
        settings.locale = "zh-Hans".into();
        constants.rebuild_path_list(None, crate::game::shared_game_state::Season::current(), &settings);
        assert_eq!(constants.base_paths[0], "/zh-Hans/");
        assert!(!constants.base_paths.contains(&"/en/".to_owned()));
        settings.locale = "en".into();
        constants.unavailable_english_pack = true;
        constants.rebuild_path_list(None, crate::game::shared_game_state::Season::current(), &settings);
        assert!(!constants.base_paths.contains(&"/en/".to_owned()), "failed optional pack must not replace intact base data");
    }

    #[test]
    fn locale_overlay_is_listed_once_and_uses_external_encoding() {
        use crate::framework::vfs::PhysicalFS;
        let root = std::env::temp_dir().join(format!("cavestory-locales-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(root.join("locale")).unwrap();
        std::fs::write(root.join("locale/en.json"),
            br#"{"name":"External English","font":"fonts/chinese-12.fnt","font_scale":"1.0","encoding":"gbk","stage_encoding":"gbk"}"#).unwrap();
        let mut ctx = Context::new();
        ctx.filesystem.mount_vfs(Box::new(PhysicalFS::new(&root, true)));
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut constants = crate::engine_constants::EngineConstants::defaults();
        constants.base_paths = vec!["/".to_owned(), "/builtin/builtin_data/".to_owned()];
        constants.load_locales(&mut ctx).unwrap();
        let english: Vec<_> = constants.locales.iter().filter(|locale| locale.code == "en").collect();
        assert_eq!(english.len(), 1);
        assert_eq!(english[0].name, "External English");
        assert_eq!(english[0].font.path, "fonts/chinese-12.fnt");
        drop(ctx);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rumble_defaults_are_enabled_and_enhanced_for_new_and_missing_settings() {
        let defaults = Settings::default();
        assert!(defaults.player1_rumble && defaults.player2_rumble);
        assert_eq!(defaults.player1_rumble_mode, RumbleMode::Enhanced);
        assert_eq!(defaults.player2_rumble_mode, RumbleMode::Enhanced);
        let mut json = serde_json::to_value(defaults).unwrap();
        for key in ["player1_rumble", "player2_rumble", "player1_rumble_mode", "player2_rumble_mode"] {
            json.as_object_mut().unwrap().remove(key);
        }
        let loaded: Settings = serde_json::from_value(json).unwrap();
        let loaded = loaded.upgrade();
        assert!(loaded.player1_rumble && loaded.player2_rumble);
        assert_eq!(loaded.player1_rumble_mode, RumbleMode::Enhanced);
        assert_eq!(loaded.player2_rumble_mode, RumbleMode::Enhanced);
    }

    #[test]
    fn rumble_modes_default_for_old_configs_without_changing_switches() {
        let mut json = serde_json::to_value(Settings::default()).unwrap();
        json.as_object_mut().unwrap().remove("player1_rumble_mode");
        json.as_object_mut().unwrap().remove("player2_rumble_mode");
        json["player1_rumble"] = serde_json::json!(true);
        json["player2_rumble"] = serde_json::json!(false);
        let loaded: Settings = serde_json::from_value(json).unwrap();
        let saved = serde_json::to_value(loaded.upgrade()).unwrap();
        assert_eq!(saved["player1_rumble_mode"], "Enhanced");
        assert_eq!(saved["player2_rumble_mode"], "Enhanced");
        assert_eq!(saved["player1_rumble"], true);
        assert_eq!(saved["player2_rumble"], false);
    }

    #[test]
    fn rumble_modes_roundtrip_independently_even_when_disabled() {
        let mut json = serde_json::to_value(Settings::default()).unwrap();
        json["player1_rumble"] = serde_json::json!(false);
        json["player1_rumble_mode"] = serde_json::json!("Enhanced");
        json["player2_rumble_mode"] = serde_json::json!("Original");
        let loaded: Settings = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(loaded.upgrade()).unwrap(), json);
    }

    struct TestGamepad;
    impl crate::framework::backend::BackendGamepad for TestGamepad {
        fn instance_id(&self) -> u32 { 73 }
        fn set_rumble(&mut self, _: u16, _: u16, _: u32) -> GameResult { Ok(()) }
    }

    #[test]
    fn player_one_accepts_hotplug_keyboard_and_live_auto_toggle() {
        let mut ctx = Context::new();
        ctx.headless = true;
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut state = SharedGameState::new(&mut ctx).unwrap();
        state.settings.touch_controls = false;
        state.settings.player1_controller_type = ControllerType::Keyboard;
        state.settings.auto_controller_switching = true;
        let mut player = state.settings.create_player1_controller();
        player.update(&mut state, &mut ctx).unwrap();
        ctx.gamepad_context.add_gamepad(Box::new(TestGamepad), 0.3);
        ctx.gamepad_context.set_button(73, Button::East, true);
        player.update(&mut state, &mut ctx).unwrap();
        player.update_trigger();
        assert!(player.trigger_jump());
        state.settings.auto_controller_switching = false;
        player.update(&mut state, &mut ctx).unwrap();
        assert!(!player.jump());
        ctx.keyboard_context.set_key(ScanCode::Z, true);
        player.update(&mut state, &mut ctx).unwrap();
        assert!(player.jump());
        ctx.keyboard_context.set_key(ScanCode::Z, false);
        state.settings.auto_controller_switching = true;
        state.settings.player2_controller_type = ControllerType::Gamepad(0);
        player.update(&mut state, &mut ctx).unwrap();
        assert!(!player.jump(), "must not take player two's pad");
        state.settings.player2_controller_type = ControllerType::Keyboard;
        ctx.gamepad_context.remove_gamepad(73);
        player.update(&mut state, &mut ctx).unwrap();
        assert!(!player.jump());
    }

    fn old_settings() -> Settings {
        let mut settings = Settings::default();
        settings.version = 25;
        let mut json = serde_json::to_value(&settings).unwrap();
        for key in ["auto_controller_switching", "player1_custom_controller_button_map", "player2_custom_controller_button_map"] {
            json.as_object_mut().unwrap().remove(key);
        }
        for player in ["player1_controller_button_map", "player2_controller_button_map"] {
            json[player]["map"] = serde_json::json!({"ButtonInput": "North"});
            json[player]["menu_ok"] = serde_json::json!({"ButtonInput": "South"});
            json[player]["menu_back"] = serde_json::json!({"ButtonInput": "East"});
        }
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn android_language_tracks_platform_changes_and_preserves_manual_choice() {
        let mut settings = Settings::default();
        settings.follow_system_language = true;
        settings.apply_android_language("system:jp");
        assert_eq!(settings.locale, "jp");
        settings.locale = "en".into();
        settings.follow_system_language = false;
        settings.apply_android_language("system:jp");
        assert_eq!(settings.locale, "en", "manual selection survives restart");
        let mut settings: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        settings.apply_android_language("app:zh-Hans");
        assert_eq!(settings.locale, "zh-Hans", "Android app language overrides previous selection");
        settings.apply_android_language("system:jp");
        assert_eq!(settings.locale, "jp", "returning to system default takes effect");
    }

    #[test]
    fn android_language_detects_changes_between_locales_with_the_same_fallback() {
        let mut settings = Settings::default();
        settings.follow_system_language = true;
        settings.apply_android_language("system:en:en-US");
        settings.locale = "jp".into();
        settings.follow_system_language = false;
        settings.apply_android_language("system:en:en-US");
        assert_eq!(settings.locale, "jp");
        settings.apply_android_language("system:en:fr-FR");
        assert_eq!(settings.locale, "en", "a new system locale selects the English fallback");
    }

    #[test]
    fn android_language_migration_keeps_old_saved_choices() {
        let mut json = serde_json::to_value(Settings::default()).unwrap();
        json["locale"] = serde_json::json!("jp");
        json.as_object_mut().unwrap().remove("follow_system_language");
        json.as_object_mut().unwrap().remove("android_language");
        let mut settings: Settings = serde_json::from_value(json.clone()).unwrap();
        settings.apply_android_language("system:zh-Hans");
        assert_eq!(settings.locale, "jp");
        let mut settings: Settings = serde_json::from_value(json).unwrap();
        settings.apply_android_language("app:en");
        assert_eq!(settings.locale, "en", "explicit Android application preference");
    }

    #[test]
    fn automatic_sensitivity_respects_player_two_assignment() {
        let mut settings = Settings::default();
        settings.player1_controller_type = ControllerType::Keyboard;
        settings.player2_controller_type = ControllerType::Gamepad(1);
        settings.player1_controller_axis_sensitivity = 0.6;
        settings.player2_controller_axis_sensitivity = 0.4;
        assert_eq!(settings.get_gamepad_axis_sensitivity(0), 0.6);
        assert_eq!(settings.get_gamepad_axis_sensitivity(1), 0.4);
        settings.auto_controller_switching = false;
        assert_eq!(settings.get_gamepad_axis_sensitivity(0), default_controller_axis_sensitivity());
    }

    #[test]
    fn upgrade_preserves_existing_layout_and_enables_auto_input() {
        let settings = old_settings().upgrade();
        let first = serde_json::to_value(&settings).unwrap();
        assert_eq!(settings.version, 26);
        assert!(settings.auto_controller_switching);
        for player in ["player1_controller_button_map", "player2_controller_button_map"] {
            assert_eq!(first[player]["menu_ok"], serde_json::json!({"ButtonInput": "South"}));
            assert_eq!(first[player]["map"], serde_json::json!({"ButtonInput": "North"}));
        }
        assert_eq!(first, serde_json::to_value(settings.upgrade()).unwrap());
    }

    #[test]
    fn upgrade_preserves_custom_bindings_and_other_settings() {
        let mut settings = old_settings();
        settings.player1_controller_button_map.jump = PlayerControllerInputType::ButtonInput(Button::LeftStick);
        let before = serde_json::to_value(&settings).unwrap();
        let after = serde_json::to_value(settings.upgrade()).unwrap();
        assert_eq!(before["player1_controller_button_map"], after["player1_controller_button_map"]);
        for key in ["player1_controller_type", "player1_rumble", "player1_key_map", "locale", "touch_controls"] {
            assert_eq!(before[key], after[key], "changed {key}");
        }
    }

    #[test]
    fn switching_presets_restores_saved_custom_mapping() {
        let mut map = player_default_controller_button_map();
        map.jump = PlayerControllerInputType::ButtonInput(Button::LeftStick);
        let original = map.clone();
        let mut custom = None;
        map.switch_layout(1, &mut custom);
        assert_eq!(map, psp_controller_button_map());
        assert_eq!(custom, Some(original.clone()));
        map.switch_layout(0, &mut custom);
        assert_eq!(map, player_default_controller_button_map());
        map.switch_layout(2, &mut custom);
        assert_eq!(map, original);
        let restored: PlayerControllerButtonMap = serde_json::from_str(&serde_json::to_string(&map).unwrap()).unwrap();
        assert_eq!(restored, original);
    }
}
