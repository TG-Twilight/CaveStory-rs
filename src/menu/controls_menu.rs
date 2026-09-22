use crate::common::Rect;
use crate::framework::context::Context;
use crate::framework::error::GameResult;
use crate::framework::gamepad::{self, Axis, AxisDirection, Button, PlayerControllerInputType};
use crate::framework::keyboard::ScanCode;
use crate::game::settings::{
    p1_default_keymap, p2_default_keymap, player_default_controller_button_map, ControllerType,
    PlayerControllerButtonMap, PlayerKeyMap, RumbleMode,
};
use crate::game::shared_game_state::SharedGameState;
use crate::input::combined_menu_controller::CombinedMenuController;

use super::{ControlMenuData, Menu, MenuEntry, MenuSelectionResult};

const FORBIDDEN_SCANCODES: [ScanCode; 12] = [
    ScanCode::F1,
    ScanCode::F2,
    ScanCode::F3,
    ScanCode::F4,
    ScanCode::F5,
    ScanCode::F6,
    ScanCode::F7,
    ScanCode::F8,
    ScanCode::F9,
    ScanCode::F10,
    ScanCode::F11,
    ScanCode::F12,
];

#[cfg(test)]
mod automatic_rebind_tests {
    use super::*;

    #[test]
    fn rumble_mode_menu_tick_stops_motor_after_disable() {
        use std::sync::{Arc, Mutex};
        struct RecordingPad(Arc<Mutex<Vec<(u16, u16, u32)>>>);
        impl crate::framework::backend::BackendGamepad for RecordingPad {
            fn instance_id(&self) -> u32 { 92 }
            fn set_rumble(&mut self, low: u16, high: u16, ms: u32) -> GameResult {
                self.0.lock().unwrap().push((low, high, ms));
                Ok(())
            }
        }
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut ctx = Context::new();
        ctx.headless = true;
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut state = SharedGameState::new(&mut ctx).unwrap();
        state.settings.auto_controller_switching = true;
        state.settings.player1_rumble = true;
        ctx.gamepad_context.add_gamepad(Box::new(RecordingPad(calls.clone())), 0.3);
        let mut menu = ControlsMenu::new();
        menu.init(&mut state, &mut ctx).unwrap();
        gamepad::set_rumble(&mut ctx, &state, 0, 1000, 2000, 50).unwrap();
        state.settings.player1_rumble = false;
        menu.tick(&mut || {}, &mut CombinedMenuController::new(), &mut state, &mut ctx).unwrap();
        assert_eq!(*calls.lock().unwrap(), vec![(1000, 2000, 1000), (0, 0, 0)]);
    }

    #[test]
    fn rumble_mode_menu_tracks_selected_player_and_hotplug() {
        let mut ctx = Context::new();
        ctx.headless = true;
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut state = SharedGameState::new(&mut ctx).unwrap();
        state.settings.player2_controller_type = ControllerType::Gamepad(0);
        state.settings.player2_rumble_mode = crate::game::settings::RumbleMode::Enhanced;
        ctx.gamepad_context.add_gamepad(Box::new(Pad), 0.3);
        let mut menu = ControlsMenu::new();
        menu.selected_player = Player::Player2;
        menu.init(&mut state, &mut ctx).unwrap();
        let mode_index = |menu: &ControlsMenu| menu.main.entries.iter().find_map(|(_, entry)| {
            if let MenuEntry::Options(label, index, _) = entry {
                if label == state.loc.t("menus.controls_menu.rumble_mode") { return Some(*index); }
            }
            None
        });
        assert_eq!(mode_index(&menu), Some(1));
        menu.selected_player = Player::Player1;
        menu.update_controller_options(&state, &ctx);
        assert_eq!(mode_index(&menu), None, "P1 must not use P2's device");
        menu.selected_player = Player::Player2;
        ctx.gamepad_context.remove_gamepad(91);
        menu.update_controller_options(&state, &ctx);
        assert_eq!(mode_index(&menu), None);
    }
    struct Pad;
    impl crate::framework::backend::BackendGamepad for Pad {
        fn instance_id(&self) -> u32 { 91 }
        fn set_rumble(&mut self, _: u16, _: u16, _: u32) -> GameResult { Ok(()) }
    }
    #[test]
    fn automatic_rebind_uses_effective_pad_not_saved_keyboard() {
        let mut ctx = Context::new();
        ctx.headless = true;
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut state = SharedGameState::new(&mut ctx).unwrap();
        state.settings.player1_controller_type = ControllerType::Keyboard;
        state.settings.auto_controller_switching = true;
        ctx.gamepad_context.add_gamepad(Box::new(Pad), 0.3);
        assert!(Player::Player1.controller_type(&state, &ctx) == ControllerType::Gamepad(0));
        state.settings.auto_controller_switching = false;
        assert!(Player::Player1.controller_type(&state, &ctx) == ControllerType::Keyboard);
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[repr(u8)]
enum CurrentMenu {
    MainMenu,
    SelectControllerMenu,
    RebindMenu,
    ConfirmRebindMenu,
    ConfirmResetMenu,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MainMenuEntry {
    SelectedPlayer,
    Controller,
    Layout,
    Rebind,
    Rumble,
    RumbleMode,
    #[cfg(all(target_os = "android", feature = "backend-sdl"))]
    ShizukuRumble,
    DisplayTouchControls,
    AutoHideTouchControls,
    Back,
}

impl Default for MainMenuEntry {
    fn default() -> Self {
        return MainMenuEntry::SelectedPlayer;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SelectControllerMenuEntry {
    Automatic,
    Keyboard,
    Gamepad(usize),
    Back,
}

impl Default for SelectControllerMenuEntry {
    fn default() -> Self {
        SelectControllerMenuEntry::Keyboard
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RebindMenuEntry {
    Control(ControlEntry),
    Reset,
    Back,
}

impl Default for RebindMenuEntry {
    fn default() -> Self {
        RebindMenuEntry::Control(ControlEntry::MenuOk)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ConfirmResetMenuEntry {
    Title,
    Yes,
    No,
}

impl Default for ConfirmResetMenuEntry {
    fn default() -> Self {
        ConfirmResetMenuEntry::No
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Player {
    Player1,
    Player2,
}

impl Player {
    fn controller_type(self, state: &SharedGameState, ctx: &Context) -> ControllerType {
        match self {
            Player::Player1 if state.settings.auto_controller_switching => state.settings
                .player1_gamepad_index(&ctx.gamepad_context).map_or(ControllerType::Keyboard, ControllerType::Gamepad),
            Player::Player1 => state.settings.player1_controller_type,
            Player::Player2 => state.settings.player2_controller_type,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum ControlEntry {
    Left,
    Up,
    Right,
    Down,
    PrevWeapon,
    NextWeapon,
    Jump,
    Shoot,
    Skip,
    Inventory,
    Map,
    Strafe,
    MenuOk,
    MenuBack,
}

impl ControlEntry {
    fn to_string(&self, state: &SharedGameState) -> String {
        match self {
            ControlEntry::Left => state.loc.t("menus.controls_menu.rebind_menu.left"),
            ControlEntry::Up => state.loc.t("menus.controls_menu.rebind_menu.up"),
            ControlEntry::Right => state.loc.t("menus.controls_menu.rebind_menu.right"),
            ControlEntry::Down => state.loc.t("menus.controls_menu.rebind_menu.down"),
            ControlEntry::PrevWeapon => state.loc.t("menus.controls_menu.rebind_menu.prev_weapon"),
            ControlEntry::NextWeapon => state.loc.t("menus.controls_menu.rebind_menu.next_weapon"),
            ControlEntry::Jump => state.loc.t("menus.controls_menu.rebind_menu.jump"),
            ControlEntry::Shoot => state.loc.t("menus.controls_menu.rebind_menu.shoot"),
            ControlEntry::Skip => state.loc.t("menus.controls_menu.rebind_menu.skip"),
            ControlEntry::Inventory => state.loc.t("menus.controls_menu.rebind_menu.inventory"),
            ControlEntry::Map => state.loc.t("menus.controls_menu.rebind_menu.map"),
            ControlEntry::Strafe => state.loc.t("menus.controls_menu.rebind_menu.strafe"),
            ControlEntry::MenuOk => state.loc.t("menus.controls_menu.rebind_menu.menu_ok"),
            ControlEntry::MenuBack => state.loc.t("menus.controls_menu.rebind_menu.menu_back"),
        }
        .to_owned()
    }
}

pub struct ControlsMenu {
    current: CurrentMenu,
    main: Menu<MainMenuEntry>,
    select_controller: Menu<SelectControllerMenuEntry>,
    rebind: Menu<RebindMenuEntry>,
    confirm_rebind: Menu<usize>,
    confirm_reset: Menu<ConfirmResetMenuEntry>,

    selected_player: Player,
    selected_controller: ControllerType,
    selected_control: Option<ControlEntry>,

    player1_key_map: Vec<(ControlEntry, ScanCode)>,
    player2_key_map: Vec<(ControlEntry, ScanCode)>,
    player1_controller_button_map: Vec<(ControlEntry, PlayerControllerInputType)>,
    player2_controller_button_map: Vec<(ControlEntry, PlayerControllerInputType)>,

    input_busy: bool,
}

impl ControlsMenu {
    pub fn new() -> ControlsMenu {
        let main = Menu::new(0, 0, 220, 0);
        let select_controller = Menu::new(0, 0, 220, 0);
        let rebind = Menu::new(0, 0, 220, 0);
        let confirm_rebind = Menu::new(0, 0, 220, 0);
        let confirm_reset = Menu::new(0, 0, 160, 0);

        ControlsMenu {
            current: CurrentMenu::MainMenu,
            main,
            select_controller,
            rebind,
            confirm_rebind,
            confirm_reset,

            selected_player: Player::Player1,
            selected_controller: ControllerType::Keyboard,
            selected_control: None,

            player1_key_map: Vec::new(),
            player2_key_map: Vec::new(),
            player1_controller_button_map: Vec::new(),
            player2_controller_button_map: Vec::new(),

            input_busy: false,
        }
    }

    pub fn init(&mut self, state: &mut SharedGameState, ctx: &mut Context) -> GameResult {
        if state.constants.supports_two_player {
            self.main.push_entry(
                MainMenuEntry::SelectedPlayer,
                MenuEntry::Options(
                    state.loc.t("menus.controls_menu.select_player.entry").to_owned(),
                    self.selected_player as usize,
                    vec![
                        state.loc.t("menus.controls_menu.select_player.player_1").to_owned(),
                        state.loc.t("menus.controls_menu.select_player.player_2").to_owned(),
                    ],
                ),
            );
        }

        self.main.push_entry(
            MainMenuEntry::Controller,
            MenuEntry::Active(state.loc.t("menus.controls_menu.controller.entry").to_owned()),
        );
        self.main
            .push_entry(MainMenuEntry::Rebind, MenuEntry::Active(state.loc.t("menus.controls_menu.rebind").to_owned()));
        self.main.push_entry(MainMenuEntry::Rumble, MenuEntry::Hidden);
        self.main.push_entry(MainMenuEntry::RumbleMode, MenuEntry::Hidden);
        #[cfg(all(target_os = "android", feature = "backend-sdl"))]
        self.main.push_entry(MainMenuEntry::ShizukuRumble,
            MenuEntry::Active(state.loc.t_optional("menus.controls_menu.rumble_output")
                .unwrap_or(match state.loc.code.as_str() {
                    "zh-Hans" => "振动输出方式", "zh-Hant" => "振動輸出方式",
                    "jp" => "振動出力方式", _ => "Rumble output",
                }).to_owned()));
        self.main.push_entry(MainMenuEntry::Layout, MenuEntry::Hidden);

        if state.settings.touch_controls {
            self.main.push_entry(
                MainMenuEntry::DisplayTouchControls,
                MenuEntry::Toggle(
                    state.loc.t("menus.options_menu.controls_menu.display_touch_controls").to_owned(),
                    state.settings.display_touch_controls,
                ),
            );
            if cfg!(target_os = "android") {
                self.main.push_entry(MainMenuEntry::AutoHideTouchControls, MenuEntry::Toggle(
                    state.loc.t("menus.options_menu.controls_menu.auto_hide_touch_controls").to_owned(),
                    state.settings.auto_hide_touch_controls,
                ));
            }
        }
        self.main.push_entry(MainMenuEntry::Back, MenuEntry::Active(state.loc.t("common.back").to_owned()));

        self.confirm_reset.push_entry(
            ConfirmResetMenuEntry::Title,
            MenuEntry::Disabled(state.loc.t("menus.controls_menu.reset_confirm_menu_title").to_owned()),
        );
        self.confirm_reset
            .push_entry(ConfirmResetMenuEntry::Yes, MenuEntry::Active(state.loc.t("common.yes").to_owned()));
        self.confirm_reset
            .push_entry(ConfirmResetMenuEntry::No, MenuEntry::Active(state.loc.t("common.no").to_owned()));

        self.player1_key_map = self.init_key_map(&state.settings.player1_key_map);
        self.player2_key_map = self.init_key_map(&state.settings.player2_key_map);
        self.player1_controller_button_map =
            self.init_controller_button_map(&state.settings.player1_controller_button_map);
        self.player2_controller_button_map =
            self.init_controller_button_map(&state.settings.player2_controller_button_map);

        self.confirm_rebind.draw_cursor = false;
        self.confirm_rebind.non_interactive = true;

        self.update_controller_options(state, ctx);
        self.update_rebind_menu(state, ctx);
        self.update_sizes(state);

        Ok(())
    }

    fn update_sizes(&mut self, state: &SharedGameState) {
        self.main.update_width(state);
        self.main.update_height(state);
        self.main.x = ((state.canvas_size.0 - self.main.width as f32) / 2.0).floor() as isize;
        self.main.y = ((state.canvas_size.1 - self.main.height as f32) / 2.0).floor() as isize;

        self.select_controller.update_width(state);
        self.select_controller.update_height(state);
        self.select_controller.x = ((state.canvas_size.0 - self.select_controller.width as f32) / 2.0).floor() as isize;
        self.select_controller.y =
            ((state.canvas_size.1 - self.select_controller.height as f32) / 2.0).floor() as isize;

        self.rebind.update_width(state);
        self.rebind.update_height(state);
        self.rebind.x = ((state.canvas_size.0 - self.rebind.width as f32) / 2.0).floor() as isize;
        self.rebind.y = ((state.canvas_size.1 - self.rebind.height as f32) / 2.0).floor() as isize;

        self.confirm_rebind.update_width(state);
        self.confirm_rebind.update_height(state);
        self.confirm_rebind.x = ((state.canvas_size.0 - self.confirm_rebind.width as f32) / 2.0).floor() as isize;
        self.confirm_rebind.y = ((state.canvas_size.1 - self.confirm_rebind.height as f32) / 2.0).floor() as isize;

        self.confirm_reset.update_width(state);
        self.confirm_reset.update_height(state);
        self.confirm_reset.x = ((state.canvas_size.0 - self.confirm_reset.width as f32) / 2.0).floor() as isize;
        self.confirm_reset.y = ((state.canvas_size.1 - self.confirm_reset.height as f32) / 2.0).floor() as isize;
    }

    fn init_key_map(&self, settings_key_map: &PlayerKeyMap) -> Vec<(ControlEntry, ScanCode)> {
        let mut map = Vec::new();

        map.push((ControlEntry::MenuOk, settings_key_map.menu_ok));
        map.push((ControlEntry::MenuBack, settings_key_map.menu_back));
        map.push((ControlEntry::Up, settings_key_map.up));
        map.push((ControlEntry::Down, settings_key_map.down));
        map.push((ControlEntry::Left, settings_key_map.left));
        map.push((ControlEntry::Right, settings_key_map.right));
        map.push((ControlEntry::Jump, settings_key_map.jump));
        map.push((ControlEntry::Shoot, settings_key_map.shoot));
        map.push((ControlEntry::PrevWeapon, settings_key_map.prev_weapon));
        map.push((ControlEntry::NextWeapon, settings_key_map.next_weapon));
        map.push((ControlEntry::Inventory, settings_key_map.inventory));
        map.push((ControlEntry::Map, settings_key_map.map));
        map.push((ControlEntry::Skip, settings_key_map.skip));
        map.push((ControlEntry::Strafe, settings_key_map.strafe));

        map
    }

    fn init_controller_button_map(
        &self,
        settings_controller_button_map: &PlayerControllerButtonMap,
    ) -> Vec<(ControlEntry, PlayerControllerInputType)> {
        let mut map = Vec::new();

        map.push((ControlEntry::MenuOk, settings_controller_button_map.menu_ok));
        map.push((ControlEntry::MenuBack, settings_controller_button_map.menu_back));
        map.push((ControlEntry::Up, settings_controller_button_map.up));
        map.push((ControlEntry::Down, settings_controller_button_map.down));
        map.push((ControlEntry::Left, settings_controller_button_map.left));
        map.push((ControlEntry::Right, settings_controller_button_map.right));
        map.push((ControlEntry::Jump, settings_controller_button_map.jump));
        map.push((ControlEntry::Shoot, settings_controller_button_map.shoot));
        map.push((ControlEntry::PrevWeapon, settings_controller_button_map.prev_weapon));
        map.push((ControlEntry::NextWeapon, settings_controller_button_map.next_weapon));
        map.push((ControlEntry::Inventory, settings_controller_button_map.inventory));
        map.push((ControlEntry::Map, settings_controller_button_map.map));
        map.push((ControlEntry::Skip, settings_controller_button_map.skip));
        map.push((ControlEntry::Strafe, settings_controller_button_map.strafe));

        map
    }

    fn update_rebind_menu(&mut self, state: &SharedGameState, ctx: &Context) {
        self.rebind.entries.clear();

        match self.selected_player {
            Player::Player1 => {
                if self.selected_controller == ControllerType::Keyboard {
                    for (k, v) in self.player1_key_map.iter() {
                        self.rebind.push_entry(
                            RebindMenuEntry::Control(*k),
                            MenuEntry::Control(
                                k.to_string(state).to_owned(),
                                ControlMenuData::String(format!("{:?}", v)),
                            ),
                        );
                    }
                } else {
                    for (k, v) in self.player1_controller_button_map.iter() {
                        let gamepad_sprite_offset = match self.selected_player.controller_type(state, ctx) {
                            ControllerType::Keyboard => 1,
                            ControllerType::Gamepad(index) => {
                                ctx.gamepad_context.get_gamepad_sprite_offset(index as usize)
                            }
                        };

                        self.rebind.push_entry(
                            RebindMenuEntry::Control(*k),
                            MenuEntry::Control(
                                k.to_string(state).to_owned(),
                                Self::binding_display(*v, gamepad_sprite_offset, state),
                            ),
                        );
                    }
                }
            }
            Player::Player2 => {
                if self.selected_controller == ControllerType::Keyboard {
                    for (k, v) in self.player2_key_map.iter() {
                        self.rebind.push_entry(
                            RebindMenuEntry::Control(*k),
                            MenuEntry::Control(
                                k.to_string(state).to_owned(),
                                ControlMenuData::String(format!("{:?}", v)),
                            ),
                        );
                    }
                } else {
                    for (k, v) in self.player2_controller_button_map.iter() {
                        let gamepad_sprite_offset = match self.selected_player.controller_type(state, ctx) {
                            ControllerType::Keyboard => 1,
                            ControllerType::Gamepad(index) => {
                                ctx.gamepad_context.get_gamepad_sprite_offset(index as usize)
                            }
                        };

                        self.rebind.push_entry(
                            RebindMenuEntry::Control(*k),
                            MenuEntry::Control(
                                k.to_string(state).to_owned(),
                                Self::binding_display(*v, gamepad_sprite_offset, state),
                            ),
                        );
                    }
                }
            }
        }

        self.rebind.push_entry(
            RebindMenuEntry::Reset,
            MenuEntry::Active(state.loc.t("menus.controls_menu.reset_confirm").to_owned()),
        );
        self.rebind.push_entry(RebindMenuEntry::Back, MenuEntry::Active(state.loc.t("common.back").to_owned()));
    }

    fn binding_display(binding: PlayerControllerInputType, offset: usize, state: &SharedGameState) -> ControlMenuData {
        match binding {
            PlayerControllerInputType::EitherButtons(first, second) => ControlMenuData::RectPair(
                first.get_rect(offset, &state.constants), second.get_rect(offset, &state.constants)),
            _ => ControlMenuData::Rect(binding.get_rect(offset, &state.constants)),
        }
    }

    fn update_controller_options(&mut self, state: &SharedGameState, ctx: &Context) {
        self.select_controller.entries.clear();

        if self.selected_player == Player::Player1 {
            self.select_controller.push_entry(
                SelectControllerMenuEntry::Automatic,
                MenuEntry::Active(state.loc.t("menus.controls_menu.controller.automatic").to_owned()),
            );
        }

        self.select_controller.push_entry(
            SelectControllerMenuEntry::Keyboard,
            MenuEntry::Active(state.loc.t(
                if state.settings.touch_controls {
                    "menus.controls_menu.controller.touch_controls_or_keyboard"
                } else {
                    "menus.controls_menu.controller.keyboard"
                }
            ).to_owned()),
        );

        let gamepads: Vec<_> = gamepad::get_gamepads(ctx).collect();

        let other_player_controller_type = match self.selected_player {
            Player::Player1 => state.settings.player2_controller_type,
            Player::Player2 if state.settings.auto_controller_switching => ControllerType::Keyboard,
            Player::Player2 => state.settings.player1_controller_type,
        };

        for &(i, gamepad) in &gamepads {
            if let ControllerType::Gamepad(index) = other_player_controller_type {
                if index as usize == i {
                    continue;
                }
            }

            self.select_controller.push_entry(
                SelectControllerMenuEntry::Gamepad(i),
                MenuEntry::Active(format!("{} {}", gamepad.get_gamepad_name(), i + 1)),
            );
        }

        self.select_controller
            .push_entry(SelectControllerMenuEntry::Back, MenuEntry::Active(state.loc.t("common.back").to_owned()));

        let controller_type = self.selected_player.controller_type(state, ctx);

        let rumble = match self.selected_player {
            Player::Player1 => state.settings.player1_rumble,
            Player::Player2 => state.settings.player2_rumble,
        };

        if let ControllerType::Gamepad(index) = controller_type {
            if ctx.gamepad_context.instance_id(index).is_none() {
                self.selected_controller = ControllerType::Keyboard;
                self.main.set_entry(MainMenuEntry::Rumble, MenuEntry::Hidden);
            } else {
                self.selected_controller = controller_type;
                self.main.set_entry(
                    MainMenuEntry::Rumble,
                    MenuEntry::Toggle(state.loc.t("menus.controls_menu.rumble").to_owned(), rumble),
                );
            }
        } else {
            self.selected_controller = controller_type;
            self.main.set_entry(MainMenuEntry::Rumble, MenuEntry::Hidden);
        }

        let mode_entry = if matches!(self.selected_controller, ControllerType::Gamepad(_)) {
            let mode = match self.selected_player {
                Player::Player1 => state.settings.player1_rumble_mode,
                Player::Player2 => state.settings.player2_rumble_mode,
            };
            MenuEntry::Options(
                state.loc.t("menus.controls_menu.rumble_mode").to_owned(),
                if mode == RumbleMode::Original { 0 } else { 1 },
                vec![
                    state.loc.t("menus.controls_menu.rumble_original").to_owned(),
                    state.loc.t("menus.controls_menu.rumble_enhanced").to_owned(),
                ],
            )
        } else {
            MenuEntry::Hidden
        };
        self.main.set_entry(MainMenuEntry::RumbleMode, mode_entry);

        match self.selected_controller {
            ControllerType::Keyboard => self.select_controller.selected = SelectControllerMenuEntry::Keyboard,
            ControllerType::Gamepad(index) => {
                self.select_controller.selected = SelectControllerMenuEntry::Gamepad(index as usize)
            }
        }
        if self.selected_player == Player::Player1 && state.settings.auto_controller_switching {
            self.select_controller.selected = SelectControllerMenuEntry::Automatic;
        }
    }

    fn update_layout_options(&mut self, state: &SharedGameState) {
        let (map, custom) = match self.selected_player {
            Player::Player1 => (&state.settings.player1_controller_button_map, &state.settings.player1_custom_controller_button_map),
            Player::Player2 => (&state.settings.player2_controller_button_map, &state.settings.player2_custom_controller_button_map),
        };
        let mut names = vec!["Xbox".to_owned(), "PSP".to_owned()];
        if custom.is_some() || map.layout_index() == 2 {
            names.push(state.loc.t("menus.controls_menu.custom_layout").to_owned());
        }
        self.main.set_entry(MainMenuEntry::Layout, MenuEntry::Options(
            state.loc.t("menus.controls_menu.layout").to_owned(), map.layout_index(), names,
        ));
    }

    fn rebuild_menu_controller(&mut self, state: &SharedGameState, controller: &mut CombinedMenuController) {
        let mut new_controller = CombinedMenuController::new();
        new_controller.add(state.settings.create_player1_controller());
        new_controller.add(state.settings.create_player2_controller());
        *controller = new_controller;
        self.input_busy = true;
        self.main.non_interactive = true;
    }

    fn switch_layout(&mut self, direction: isize, state: &mut SharedGameState, ctx: &Context,
                     controller: &mut CombinedMenuController) -> GameResult {
        let (map, custom) = match self.selected_player {
            Player::Player1 => (&mut state.settings.player1_controller_button_map, &mut state.settings.player1_custom_controller_button_map),
            Player::Player2 => (&mut state.settings.player2_controller_button_map, &mut state.settings.player2_custom_controller_button_map),
        };
        let count = if custom.is_some() || map.layout_index() == 2 { 3 } else { 2 };
        let next = (map.layout_index() as isize + direction).rem_euclid(count) as usize;
        map.switch_layout(next, custom);
        self.player1_controller_button_map = self.init_controller_button_map(&state.settings.player1_controller_button_map);
        self.player2_controller_button_map = self.init_controller_button_map(&state.settings.player2_controller_button_map);
        self.update_rebind_menu(state, ctx);
        self.rebuild_menu_controller(state, controller);
        state.settings.save(ctx)
    }

    fn update_confirm_controls_menu(&mut self, state: &SharedGameState) {
        match self.selected_control {
            Some(control) => {
                self.confirm_rebind.entries.clear();

                self.confirm_rebind.push_entry(
                    0,
                    MenuEntry::DisabledWhite(state.tt(
                        "menus.controls_menu.rebind_confirm_menu.title",
                        &[("control", control.to_string(state).as_str())],
                    )),
                );
                self.confirm_rebind.push_entry(
                    1,
                    MenuEntry::Disabled(state.loc.t(
                        if state.settings.touch_controls {
                            "menus.controls_menu.rebind_confirm_menu.cancel_touch"
                        } else {
                            "menus.controls_menu.rebind_confirm_menu.cancel"
                        }
                    ).to_owned()),
                );
            }
            None => {}
        }
    }

    fn reset_controls(&mut self, state: &mut SharedGameState, ctx: &Context) -> GameResult {
        match self.selected_player {
            Player::Player1 => {
                if self.selected_controller == ControllerType::Keyboard {
                    state.settings.player1_key_map = p1_default_keymap();
                    self.player1_key_map = self.init_key_map(&state.settings.player1_key_map);
                } else {
                    state.settings.player1_controller_button_map = player_default_controller_button_map();
                    self.player1_controller_button_map =
                        self.init_controller_button_map(&state.settings.player1_controller_button_map);
                }
            }
            Player::Player2 => {
                if self.selected_controller == ControllerType::Keyboard {
                    state.settings.player2_key_map = p2_default_keymap();
                    self.player2_key_map = self.init_key_map(&state.settings.player2_key_map);
                } else {
                    state.settings.player2_controller_button_map = player_default_controller_button_map();
                    self.player2_controller_button_map =
                        self.init_controller_button_map(&state.settings.player2_controller_button_map);
                }
            }
        }

        state.settings.save(ctx)
    }

    fn is_key_occupied(&self, scan_code: ScanCode) -> bool {
        let other_player_keymap = match self.selected_player {
            Player::Player1 => &self.player2_key_map,
            Player::Player2 => &self.player1_key_map,
        };

        for (_, v) in other_player_keymap.iter() {
            if *v == scan_code {
                return true;
            }
        }

        false
    }

    fn set_key(&mut self, state: &mut SharedGameState, scan_code: ScanCode, ctx: &Context) -> GameResult {
        if self.selected_control.is_none() {
            return Ok(());
        }

        let mut did_swap_controls = false;

        match self.selected_control.unwrap() {
            ControlEntry::Left => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.left = scan_code,
                Player::Player2 => state.settings.player2_key_map.left = scan_code,
            },
            ControlEntry::Up => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.up = scan_code,
                Player::Player2 => state.settings.player2_key_map.up = scan_code,
            },
            ControlEntry::Right => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.right = scan_code,
                Player::Player2 => state.settings.player2_key_map.right = scan_code,
            },
            ControlEntry::Down => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.down = scan_code,
                Player::Player2 => state.settings.player2_key_map.down = scan_code,
            },
            ControlEntry::PrevWeapon => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.prev_weapon = scan_code,
                Player::Player2 => state.settings.player2_key_map.prev_weapon = scan_code,
            },
            ControlEntry::NextWeapon => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.next_weapon = scan_code,
                Player::Player2 => state.settings.player2_key_map.next_weapon = scan_code,
            },
            ControlEntry::Jump => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_key_map.jump,
                        &mut state.settings.player1_key_map.shoot,
                        scan_code,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_key_map.jump,
                        &mut state.settings.player2_key_map.shoot,
                        scan_code,
                    );
                }
            },
            ControlEntry::Shoot => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_key_map.shoot,
                        &mut state.settings.player1_key_map.jump,
                        scan_code,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_key_map.shoot,
                        &mut state.settings.player2_key_map.jump,
                        scan_code,
                    );
                }
            },
            ControlEntry::Skip => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.skip = scan_code,
                Player::Player2 => state.settings.player2_key_map.skip = scan_code,
            },
            ControlEntry::Inventory => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.inventory = scan_code,
                Player::Player2 => state.settings.player2_key_map.inventory = scan_code,
            },
            ControlEntry::Map => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.map = scan_code,
                Player::Player2 => state.settings.player2_key_map.map = scan_code,
            },
            ControlEntry::Strafe => match self.selected_player {
                Player::Player1 => state.settings.player1_key_map.strafe = scan_code,
                Player::Player2 => state.settings.player2_key_map.strafe = scan_code,
            },
            ControlEntry::MenuOk => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_key_map.menu_ok,
                        &mut state.settings.player1_key_map.menu_back,
                        scan_code,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_key_map.menu_ok,
                        &mut state.settings.player2_key_map.menu_back,
                        scan_code,
                    );
                }
            },
            ControlEntry::MenuBack => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_key_map.menu_back,
                        &mut state.settings.player1_key_map.menu_ok,
                        scan_code,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_key_map.menu_back,
                        &mut state.settings.player2_key_map.menu_ok,
                        scan_code,
                    );
                }
            },
        }

        state.settings.save(ctx)?;

        let keymap = match self.selected_player {
            Player::Player1 => &mut self.player1_key_map,
            Player::Player2 => &mut self.player2_key_map,
        };

        for (entry, value) in keymap.iter_mut() {
            if *entry == self.selected_control.unwrap() {
                *value = scan_code;
            }

            if did_swap_controls {
                let map = match self.selected_player {
                    Player::Player1 => &state.settings.player1_key_map,
                    Player::Player2 => &state.settings.player2_key_map,
                };

                match *entry {
                    ControlEntry::Jump => *value = map.jump,
                    ControlEntry::Shoot => *value = map.shoot,
                    ControlEntry::MenuOk => *value = map.menu_ok,
                    ControlEntry::MenuBack => *value = map.menu_back,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn set_controller_input(
        &mut self,
        state: &mut SharedGameState,
        input_type: PlayerControllerInputType,
        ctx: &Context,
    ) -> GameResult {
        if self.selected_control.is_none() {
            return Ok(());
        }

        let mut did_swap_controls = false;

        match self.selected_control.unwrap() {
            ControlEntry::Left => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.left = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.left = input_type,
            },
            ControlEntry::Up => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.up = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.up = input_type,
            },
            ControlEntry::Right => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.right = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.right = input_type,
            },
            ControlEntry::Down => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.down = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.down = input_type,
            },
            ControlEntry::PrevWeapon => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.prev_weapon = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.prev_weapon = input_type,
            },
            ControlEntry::NextWeapon => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.next_weapon = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.next_weapon = input_type,
            },
            ControlEntry::Jump => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_controller_button_map.jump,
                        &mut state.settings.player1_controller_button_map.shoot,
                        input_type,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_controller_button_map.jump,
                        &mut state.settings.player2_controller_button_map.shoot,
                        input_type,
                    );
                }
            },
            ControlEntry::Shoot => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_controller_button_map.shoot,
                        &mut state.settings.player1_controller_button_map.jump,
                        input_type,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_controller_button_map.shoot,
                        &mut state.settings.player2_controller_button_map.jump,
                        input_type,
                    );
                }
            },
            ControlEntry::Skip => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.skip = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.skip = input_type,
            },
            ControlEntry::Inventory => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.inventory = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.inventory = input_type,
            },
            ControlEntry::Map => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.map = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.map = input_type,
            },
            ControlEntry::Strafe => match self.selected_player {
                Player::Player1 => state.settings.player1_controller_button_map.strafe = input_type,
                Player::Player2 => state.settings.player2_controller_button_map.strafe = input_type,
            },
            ControlEntry::MenuOk => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_controller_button_map.menu_ok,
                        &mut state.settings.player1_controller_button_map.menu_back,
                        input_type,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_controller_button_map.menu_ok,
                        &mut state.settings.player2_controller_button_map.menu_back,
                        input_type,
                    );
                }
            },
            ControlEntry::MenuBack => match self.selected_player {
                Player::Player1 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player1_controller_button_map.menu_back,
                        &mut state.settings.player1_controller_button_map.menu_ok,
                        input_type,
                    );
                }
                Player::Player2 => {
                    did_swap_controls = self.swap_if_same(
                        &mut state.settings.player2_controller_button_map.menu_back,
                        &mut state.settings.player2_controller_button_map.menu_ok,
                        input_type,
                    );
                }
            },
        }

        state.settings.save(ctx)?;

        let button_map = match self.selected_player {
            Player::Player1 => &mut self.player1_controller_button_map,
            Player::Player2 => &mut self.player2_controller_button_map,
        };

        for (entry, value) in button_map.iter_mut() {
            if *entry == self.selected_control.unwrap() {
                *value = input_type;
            }

            if did_swap_controls {
                let map = match self.selected_player {
                    Player::Player1 => &state.settings.player1_controller_button_map,
                    Player::Player2 => &state.settings.player2_controller_button_map,
                };

                match *entry {
                    ControlEntry::Jump => *value = map.jump,
                    ControlEntry::Shoot => *value = map.shoot,
                    ControlEntry::MenuOk => *value = map.menu_ok,
                    ControlEntry::MenuBack => *value = map.menu_back,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn swap_if_same<T: Eq + Copy>(&mut self, fst: &mut T, snd: &mut T, value: T) -> bool {
        let mut swapped = false;

        if *snd == value {
            *snd = *fst;
            swapped = true;
        }

        *fst = value;

        swapped
    }

    fn normalize_gamepad_input(&self, input: PlayerControllerInputType) -> PlayerControllerInputType {
        match input {
            PlayerControllerInputType::ButtonInput(Button::DPadUp) => {
                PlayerControllerInputType::Either(Button::DPadUp, Axis::LeftY, AxisDirection::Up)
            }
            PlayerControllerInputType::ButtonInput(Button::DPadDown) => {
                PlayerControllerInputType::Either(Button::DPadDown, Axis::LeftY, AxisDirection::Down)
            }
            PlayerControllerInputType::ButtonInput(Button::DPadLeft) => {
                PlayerControllerInputType::Either(Button::DPadLeft, Axis::LeftX, AxisDirection::Left)
            }
            PlayerControllerInputType::ButtonInput(Button::DPadRight) => {
                PlayerControllerInputType::Either(Button::DPadRight, Axis::LeftX, AxisDirection::Right)
            }
            PlayerControllerInputType::AxisInput(Axis::LeftY, AxisDirection::Up) => {
                PlayerControllerInputType::Either(Button::DPadUp, Axis::LeftY, AxisDirection::Up)
            }
            PlayerControllerInputType::AxisInput(Axis::LeftY, AxisDirection::Down) => {
                PlayerControllerInputType::Either(Button::DPadDown, Axis::LeftY, AxisDirection::Down)
            }
            PlayerControllerInputType::AxisInput(Axis::LeftX, AxisDirection::Left) => {
                PlayerControllerInputType::Either(Button::DPadLeft, Axis::LeftX, AxisDirection::Left)
            }
            PlayerControllerInputType::AxisInput(Axis::LeftX, AxisDirection::Right) => {
                PlayerControllerInputType::Either(Button::DPadRight, Axis::LeftX, AxisDirection::Right)
            }
            PlayerControllerInputType::AxisInput(Axis::RightY, AxisDirection::Up) => {
                PlayerControllerInputType::Either(Button::DPadUp, Axis::RightY, AxisDirection::Up)
            }
            PlayerControllerInputType::AxisInput(Axis::RightY, AxisDirection::Down) => {
                PlayerControllerInputType::Either(Button::DPadDown, Axis::RightY, AxisDirection::Down)
            }
            PlayerControllerInputType::AxisInput(Axis::RightX, AxisDirection::Left) => {
                PlayerControllerInputType::Either(Button::DPadLeft, Axis::RightX, AxisDirection::Left)
            }
            PlayerControllerInputType::AxisInput(Axis::RightX, AxisDirection::Right) => {
                PlayerControllerInputType::Either(Button::DPadRight, Axis::RightX, AxisDirection::Right)
            }
            _ => input,
        }
    }

    pub fn tick(
        &mut self,
        exit_action: &mut dyn FnMut(),
        controller: &mut CombinedMenuController,
        state: &mut SharedGameState,
        ctx: &mut Context,
    ) -> GameResult {
        if self.current == CurrentMenu::MainMenu {
            if cfg!(target_os = "android") && state.settings.touch_controls {
                self.main.set_entry(MainMenuEntry::DisplayTouchControls,
                    if state.touch_controls.visibility.has_external_devices() {
                        MenuEntry::Toggle(state.loc.t("menus.options_menu.controls_menu.display_touch_controls").to_owned(),
                            state.settings.display_touch_controls)
                    } else {
                        MenuEntry::Disabled(state.loc.t("menus.options_menu.controls_menu.touch_controls_required").to_owned())
                    });
            }
            self.update_controller_options(state, ctx);
            self.update_rebind_menu(state, ctx);
        }
        self.update_layout_options(state);
        self.update_sizes(state);

        match self.current {
            CurrentMenu::MainMenu => match self.main.tick(controller, state) {
                MenuSelectionResult::Selected(MainMenuEntry::Layout, _)
                | MenuSelectionResult::Right(MainMenuEntry::Layout, _, _) => {
                    self.switch_layout(1, state, ctx, controller)?;
                }
                MenuSelectionResult::Left(MainMenuEntry::Layout, _, _) => {
                    self.switch_layout(-1, state, ctx, controller)?;
                }
                MenuSelectionResult::Selected(MainMenuEntry::SelectedPlayer, toggle)
                | MenuSelectionResult::Left(MainMenuEntry::SelectedPlayer, toggle, _)
                | MenuSelectionResult::Right(MainMenuEntry::SelectedPlayer, toggle, _) => {
                    if let MenuEntry::Options(_, value, _) = toggle {
                        let (new_player, new_value) = match *value {
                            0 => (Player::Player2, 1),
                            1 => (Player::Player1, 0),
                            _ => unreachable!(),
                        };

                        *value = new_value;

                        self.selected_player = new_player;
                        self.selected_controller = new_player.controller_type(state, ctx);

                        self.update_controller_options(state, ctx);
                        self.update_rebind_menu(state, ctx);
                    }
                }
                MenuSelectionResult::Selected(MainMenuEntry::Controller, _) => {
                    if self.input_busy {
                        return Ok(());
                    }

                    self.update_controller_options(state, ctx);
                    self.current = CurrentMenu::SelectControllerMenu;
                }
                MenuSelectionResult::Selected(MainMenuEntry::Rebind, _) => {
                    self.current = CurrentMenu::RebindMenu;
                }
                #[cfg(all(target_os = "android", feature = "backend-sdl"))]
                MenuSelectionResult::Selected(MainMenuEntry::ShizukuRumble, _) => {
                    ctx.stop_rumble_for_lifecycle();
                    let player = match self.selected_player { Player::Player1 => 0, Player::Player2 => 1 };
                    let allowed = ctx.gamepad_context.rumble_indices(state)[player].is_some();
                    let instance = match self.selected_player.controller_type(state, ctx) {
                        ControllerType::Gamepad(index) => ctx.gamepad_context.instance_id(index),
                        _ => None,
                    };
                    let map = match self.selected_player {
                        Player::Player1 => &state.settings.player1_controller_button_map,
                        Player::Player2 => &state.settings.player2_controller_button_map,
                    };
                    let android_key = |input: PlayerControllerInputType, fallback| match input {
                        PlayerControllerInputType::ButtonInput(button)
                        | PlayerControllerInputType::EitherButtons(button, _) => match button {
                            Button::South => 96, Button::East => 97, Button::West => 99, Button::North => 100,
                            Button::LeftShoulder => 102, Button::RightShoulder => 103,
                            Button::LeftStick => 106, Button::RightStick => 107,
                            Button::Start => 108, Button::Back => 109, _ => fallback,
                        },
                        _ => fallback,
                    };
                    crate::framework::android_rumble::show_settings(player as i32 + 1, instance, allowed,
                        android_key(map.menu_ok, 96), android_key(map.menu_back, 97))?;
                }
                MenuSelectionResult::Selected(MainMenuEntry::RumbleMode, _)
                | MenuSelectionResult::Left(MainMenuEntry::RumbleMode, _, _)
                | MenuSelectionResult::Right(MainMenuEntry::RumbleMode, _, _) => {
                    let mode = match self.selected_player {
                        Player::Player1 => &mut state.settings.player1_rumble_mode,
                        Player::Player2 => &mut state.settings.player2_rumble_mode,
                    };
                    *mode = match *mode {
                        RumbleMode::Original => RumbleMode::Enhanced,
                        RumbleMode::Enhanced => RumbleMode::Original,
                    };
                    state.settings.save(ctx)?;
                    self.update_controller_options(state, ctx);
                }
                MenuSelectionResult::Selected(MainMenuEntry::Rumble, toggle) => {
                    if let MenuEntry::Toggle(_, value) = toggle {
                        match self.selected_player {
                            Player::Player1 => {
                                state.settings.player1_rumble = !state.settings.player1_rumble;

                                if state.settings.player1_rumble {
                                    if let ControllerType::Gamepad(idx) = self.selected_controller {
                                        gamepad::set_rumble(
                                            ctx,
                                            state,
                                            idx,
                                            0,
                                            0x5000,
                                            (state.settings.timing_mode.get_tps() / 2) as u32,
                                        )?;
                                    }
                                }

                                *value = state.settings.player1_rumble;
                            }
                            Player::Player2 => {
                                state.settings.player2_rumble = !state.settings.player2_rumble;

                                if state.settings.player2_rumble {
                                    if let ControllerType::Gamepad(idx) = self.selected_controller {
                                        gamepad::set_rumble(
                                            ctx,
                                            state,
                                            idx,
                                            0,
                                            0x5000,
                                            (state.settings.timing_mode.get_tps() / 2) as u32,
                                        )?;
                                    }
                                }

                                *value = state.settings.player2_rumble;
                            }
                        }

                        state.settings.save(ctx)?;
                    }
                }
                MenuSelectionResult::Selected(MainMenuEntry::DisplayTouchControls, toggle) => {
                    if let MenuEntry::Toggle(_, value) = toggle {
                        state.settings.display_touch_controls = !state.settings.display_touch_controls;
                        let _ = state.settings.save(ctx);

                        *value = state.settings.display_touch_controls;
                    }
                }
                MenuSelectionResult::Selected(MainMenuEntry::AutoHideTouchControls, toggle) => {
                    if let MenuEntry::Toggle(_, value) = toggle {
                        state.settings.auto_hide_touch_controls = !state.settings.auto_hide_touch_controls;
                        state.touch_controls.visibility.activity(std::time::Instant::now());
                        state.settings.save(ctx)?;
                        *value = state.settings.auto_hide_touch_controls;
                    }
                }
                MenuSelectionResult::Selected(MainMenuEntry::Back, _) | MenuSelectionResult::Canceled => exit_action(),
                _ => {}
            },
            CurrentMenu::SelectControllerMenu => match self.select_controller.tick(controller, state) {
                MenuSelectionResult::Selected(SelectControllerMenuEntry::Automatic, _) => {
                    state.settings.auto_controller_switching = true;
                    state.settings.save(ctx)?;
                    self.rebuild_menu_controller(state, controller);
                    self.update_controller_options(state, ctx);
                    self.update_rebind_menu(state, ctx);
                    self.current = CurrentMenu::MainMenu;
                }
                MenuSelectionResult::Selected(SelectControllerMenuEntry::Keyboard, _) => {
                    if self.selected_player == Player::Player1 {
                        state.settings.auto_controller_switching = false;
                        state.settings.player1_controller_type = ControllerType::Keyboard;
                    } else {
                        state.settings.player2_controller_type = ControllerType::Keyboard;
                    }

                    let _ = state.settings.save(ctx);

                    let mut new_menu_controller = CombinedMenuController::new();
                    new_menu_controller.add(state.settings.create_player1_controller());
                    new_menu_controller.add(state.settings.create_player2_controller());
                    self.input_busy = true;
                    self.main.non_interactive = true;
                    *controller = new_menu_controller;

                    self.selected_controller = ControllerType::Keyboard;
                    self.update_rebind_menu(state, ctx);

                    self.current = CurrentMenu::MainMenu;
                }
                MenuSelectionResult::Selected(SelectControllerMenuEntry::Gamepad(idx), _) => {
                    if self.selected_player == Player::Player1 {
                        state.settings.auto_controller_switching = false;
                        state.settings.player1_controller_type = ControllerType::Gamepad(idx as u32);
                    } else {
                        state.settings.player2_controller_type = ControllerType::Gamepad(idx as u32);
                    }

                    let _ = state.settings.save(ctx);

                    let mut new_menu_controller = CombinedMenuController::new();
                    new_menu_controller.add(state.settings.create_player1_controller());
                    new_menu_controller.add(state.settings.create_player2_controller());
                    self.input_busy = true;
                    self.main.non_interactive = true;
                    *controller = new_menu_controller;

                    self.selected_controller = ControllerType::Gamepad(idx as u32);
                    self.update_rebind_menu(state, ctx);

                    self.current = CurrentMenu::MainMenu;
                }
                MenuSelectionResult::Selected(SelectControllerMenuEntry::Back, _) | MenuSelectionResult::Canceled => {
                    self.current = CurrentMenu::MainMenu;
                }
                _ => {}
            },
            CurrentMenu::RebindMenu => match self.rebind.tick(controller, state) {
                MenuSelectionResult::Selected(RebindMenuEntry::Back, _) | MenuSelectionResult::Canceled => {
                    if !self.input_busy {
                        self.current = CurrentMenu::MainMenu;
                    }
                }
                MenuSelectionResult::Selected(RebindMenuEntry::Control(control), _) => {
                    if !self.input_busy {
                        self.selected_control = Some(control);
                        self.update_confirm_controls_menu(state);
                        self.input_busy = true;
                        self.current = CurrentMenu::ConfirmRebindMenu;
                    }
                }
                MenuSelectionResult::Selected(RebindMenuEntry::Reset, _) => {
                    self.confirm_reset.selected = ConfirmResetMenuEntry::default();
                    self.current = CurrentMenu::ConfirmResetMenu;
                }
                _ => {}
            },
            CurrentMenu::ConfirmRebindMenu => match self.confirm_rebind.tick(controller, state) {
                _ => {
                    let entry_bounds = Rect::new_size(0, 0, ctx.screen_size.0 as isize, ctx.screen_size.1 as isize);
                    let pressed_keys: Vec<_> = ctx.keyboard_context.pressed_keys().into_iter().collect();

                    if state.touch_controls.consume_click_in(entry_bounds) {
                        state.sound_manager.play_sfx(5);
                        self.current = CurrentMenu::RebindMenu;
                        return Ok(());
                    }

                    for key in pressed_keys.clone() {
                        if *key == ScanCode::Escape {
                            state.sound_manager.play_sfx(5);
                            self.current = CurrentMenu::RebindMenu;
                            return Ok(());
                        }
                    }

                    match self.selected_controller {
                        ControllerType::Keyboard => {
                            if pressed_keys.len() == 1 {
                                if !self.input_busy {
                                    self.input_busy = true;
                                    self.rebind.non_interactive = true;

                                    let key = **pressed_keys.first().unwrap();

                                    if self.is_key_occupied(key)
                                        || FORBIDDEN_SCANCODES.contains(&key)
                                        || self.selected_controller != ControllerType::Keyboard
                                    {
                                        state.sound_manager.play_sfx(12);
                                    } else {
                                        self.set_key(state, key, ctx)?;
                                        self.update_rebind_menu(state, ctx);
                                        self.selected_control = None;
                                        state.sound_manager.play_sfx(18);
                                        self.current = CurrentMenu::RebindMenu;
                                    }
                                }
                            }
                        }
                        ControllerType::Gamepad(idx) => {
                            let pressed_gamepad_buttons: Vec<_> =
                                ctx.gamepad_context.pressed_buttons(idx).into_iter().collect();

                            for button in pressed_gamepad_buttons.clone() {
                                if button == Button::Start {
                                    state.sound_manager.play_sfx(5);
                                    self.current = CurrentMenu::RebindMenu;
                                    return Ok(());
                                }
                            }

                            if pressed_gamepad_buttons.len() == 1 {
                                if !self.input_busy {
                                    self.input_busy = true;
                                    self.rebind.non_interactive = true;

                                    let button = *pressed_gamepad_buttons.first().unwrap();

                                    if self.selected_player.controller_type(state, ctx) != self.selected_controller {
                                        state.sound_manager.play_sfx(12);
                                    } else {
                                        let normalized_input = self
                                            .normalize_gamepad_input(PlayerControllerInputType::ButtonInput(button));

                                        self.set_controller_input(state, normalized_input, ctx)?;
                                        self.update_rebind_menu(state, ctx);
                                        self.selected_control = None;
                                        state.sound_manager.play_sfx(18);
                                        self.current = CurrentMenu::RebindMenu;
                                    }
                                }
                            }

                            let active_axes: Vec<_> = ctx.gamepad_context.active_axes(idx).into_iter().collect();

                            if active_axes.len() == 1 {
                                if !self.input_busy {
                                    self.input_busy = true;
                                    self.rebind.non_interactive = true;

                                    if self.selected_player.controller_type(state, ctx) != self.selected_controller {
                                        state.sound_manager.play_sfx(12);
                                    } else {
                                        let (axis, value) = *active_axes.first().unwrap();
                                        let direction = AxisDirection::from_axis_data(axis, value);
                                        let normalized_input = self.normalize_gamepad_input(
                                            PlayerControllerInputType::AxisInput(axis, direction),
                                        );

                                        self.set_controller_input(state, normalized_input, ctx)?;
                                        self.update_rebind_menu(state, ctx);
                                        self.selected_control = None;
                                        state.sound_manager.play_sfx(18);
                                        self.current = CurrentMenu::RebindMenu;
                                    }
                                }
                            }

                            if pressed_keys.is_empty() && pressed_gamepad_buttons.is_empty() && active_axes.is_empty() {
                                self.input_busy = false;
                            }
                        }
                    }
                }
            },
            CurrentMenu::ConfirmResetMenu => match self.confirm_reset.tick(controller, state) {
                MenuSelectionResult::Selected(ConfirmResetMenuEntry::Yes, _) => {
                    self.reset_controls(state, ctx)?;
                    self.update_rebind_menu(state, ctx);
                    self.input_busy = true;
                    self.rebind.non_interactive = true;
                    self.current = CurrentMenu::RebindMenu;
                }
                MenuSelectionResult::Selected(ConfirmResetMenuEntry::No, _) | MenuSelectionResult::Canceled => {
                    self.current = CurrentMenu::RebindMenu;
                }
                _ => {}
            },
        }

        // Apply switch, mode and device assignment changes even while gameplay is paused.
        ctx.gamepad_context.reconcile_rumble(state)?;

        if self.input_busy {
            let pressed_keys = ctx.keyboard_context.pressed_keys();
            let mut input_busy = pressed_keys.len() > 0;

            let gamepads = ctx.gamepad_context.get_gamepads();
            for (idx, _) in gamepads {
                let pressed_gamepad_buttons = ctx.gamepad_context.pressed_buttons(idx as u32);
                let active_axes = ctx.gamepad_context.active_axes(idx as u32);

                input_busy = input_busy || !pressed_gamepad_buttons.is_empty() || !active_axes.is_empty();
            }

            self.input_busy = input_busy;

            if !self.input_busy {
                self.main.non_interactive = false;
                self.rebind.non_interactive = false;
            }
        }

        Ok(())
    }

    pub fn draw(&self, state: &mut SharedGameState, ctx: &mut Context) -> GameResult {
        match self.current {
            CurrentMenu::MainMenu => self.main.draw(state, ctx)?,
            CurrentMenu::SelectControllerMenu => self.select_controller.draw(state, ctx)?,
            CurrentMenu::RebindMenu => self.rebind.draw(state, ctx)?,
            CurrentMenu::ConfirmRebindMenu => self.confirm_rebind.draw(state, ctx)?,
            CurrentMenu::ConfirmResetMenu => self.confirm_reset.draw(state, ctx)?,
        }

        Ok(())
    }
}
