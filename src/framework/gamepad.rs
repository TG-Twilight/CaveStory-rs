use std::collections::{HashMap, HashSet};
use std::time::Instant;
use crate::game::rumble::{RumbleEffect, RumblePlayback};
use crate::game::settings::{ControllerType, RumbleMode};
use crate::game::shared_game_state::PlayerCount;

use serde::{Deserialize, Serialize};

use crate::framework::backend::BackendGamepad;
use crate::framework::error::GameResult;
use crate::game::shared_game_state::SharedGameState;
use crate::{common::Rect, engine_constants::EngineConstants, framework::context::Context};

const QUAKE_RUMBLE_LOW_FREQ: u16 = 0x3000;
const QUAKE_RUMBLE_HI_FREQ: u16 = 0;
const SUPER_QUAKE_RUMBLE_LOW_FREQ: u16 = 0x5000;
const SUPER_QUAKE_RUMBLE_HI_FREQ: u16 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum GamepadType {
    Unknown,
    Xbox360,
    XboxOne,
    PS3,
    PS4,
    NintendoSwitchPro,
    Virtual,
    PS5,
    AmazonLuma,
    GoogleStadia,
    NVIDIAShield,
    NintendoSwitchJoyConLeft,
    NintendoSwitchJoyConRight,
    NintendoSwitchJoyConPair,
}

impl GamepadType {
    pub fn get_name(&self) -> &str {
        match self {
            GamepadType::Unknown => "Unknown controller",
            GamepadType::Xbox360 => "Xbox 360 controller",
            GamepadType::XboxOne => "Xbox One controller",
            GamepadType::PS3 => "PlayStation 3 controller",
            GamepadType::PS4 => "PlayStation 4 controller",
            GamepadType::NintendoSwitchPro => "Nintendo Switch Pro controller",
            GamepadType::Virtual => "Virtual controller",
            GamepadType::PS5 => "PlayStation 5 controller",
            GamepadType::AmazonLuma => "Amazon Luma controller",
            GamepadType::GoogleStadia => "Google Stadia controller",
            GamepadType::NVIDIAShield => "NVIDIA Shield controller",
            GamepadType::NintendoSwitchJoyConLeft => "Nintendo Switch Joy-Con (left)",
            GamepadType::NintendoSwitchJoyConRight => "Nintendo Switch Joy-Con (right)",
            GamepadType::NintendoSwitchJoyConPair => "Nintendo Switch Joy-Con (pair)",
        }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[repr(u32)]
pub enum Axis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    TriggerLeft,
    TriggerRight,
}

impl Axis {
    pub fn get_rect(&self, offset: usize, constants: &EngineConstants) -> Rect<u16> {
        constants.gamepad.axis_rects.get(self).unwrap()[offset]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AxisDirection {
    None,
    Either,
    Up,
    Left,
    Right,
    Down,
}

impl AxisDirection {
    pub fn from_axis_data(axis: Axis, value: f64) -> Self {
        match axis {
            Axis::LeftX | Axis::RightX => {
                if value < 0.0 {
                    AxisDirection::Left
                } else {
                    AxisDirection::Right
                }
            }
            Axis::LeftY | Axis::RightY => {
                if value < 0.0 {
                    AxisDirection::Up
                } else {
                    AxisDirection::Down
                }
            }
            Axis::TriggerLeft | Axis::TriggerRight => AxisDirection::Either,
        }
    }

    pub fn compare(&self, value: f64, axis_sensitivity: f64) -> bool {
        match self {
            AxisDirection::None => false,
            AxisDirection::Either => value.abs() > 0.0,
            AxisDirection::Down | AxisDirection::Right => value > axis_sensitivity,
            AxisDirection::Up | AxisDirection::Left => value < -axis_sensitivity,
        }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[repr(u32)]
pub enum Button {
    South,
    East,
    West,
    North,

    Back,
    Guide,
    Start,
    LeftStick,
    RightStick,
    LeftShoulder,
    RightShoulder,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

impl Button {
    pub fn get_rect(&self, offset: usize, constants: &EngineConstants) -> Rect<u16> {
        match self {
            Button::Guide => Rect::new(0, 0, 0, 0),
            _ => constants.gamepad.button_rects.get(self).unwrap()[offset],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PlayerControllerInputType {
    ButtonInput(Button),
    EitherButtons(Button, Button),
    AxisInput(Axis, AxisDirection),
    Either(Button, Axis, AxisDirection),
}

impl PlayerControllerInputType {
    pub fn get_rect(&self, offset: usize, constants: &EngineConstants) -> Rect<u16> {
        match self {
            PlayerControllerInputType::ButtonInput(button) => button.get_rect(offset, constants),
            PlayerControllerInputType::EitherButtons(button, _) => button.get_rect(offset, constants),
            PlayerControllerInputType::AxisInput(axis, _) => axis.get_rect(offset, constants),
            PlayerControllerInputType::Either(button, axis, _) => button.get_rect(offset, constants),
        }
    }
}

pub struct GamepadData {
    rumble: RumblePlayback,
    last_input: u64,
    controller: Box<dyn BackendGamepad>,
    controller_type: GamepadType,

    left_x: f64,
    left_y: f64,
    right_x: f64,
    right_y: f64,
    trigger_left: f64,
    trigger_right: f64,

    axis_sensitivity: f64,

    pressed_buttons_set: HashSet<Button>,
    axis_values: HashMap<Axis, f64>,
}

impl GamepadData {
    pub(crate) fn new(game_controller: Box<dyn BackendGamepad>, axis_sensitivity: f64) -> Self {
        GamepadData {
            rumble: RumblePlayback::default(),
            last_input: 0,
            controller: game_controller,
            controller_type: GamepadType::Unknown,

            left_x: 0.0,
            left_y: 0.0,
            right_x: 0.0,
            right_y: 0.0,
            trigger_left: 0.0,
            trigger_right: 0.0,

            axis_sensitivity,

            pressed_buttons_set: HashSet::with_capacity(16),
            axis_values: HashMap::with_capacity(8),
        }
    }

    pub(crate) fn set_gamepad_type(&mut self, controller_type: GamepadType) {
        self.controller_type = controller_type;
    }

    pub(crate) fn get_gamepad_sprite_offset(&self) -> usize {
        match self.controller_type {
            GamepadType::PS3 | GamepadType::PS4 | GamepadType::PS5 => 0,
            GamepadType::Xbox360 | GamepadType::XboxOne => 1,
            GamepadType::NintendoSwitchPro
            | GamepadType::NintendoSwitchJoyConLeft
            | GamepadType::NintendoSwitchJoyConRight
            | GamepadType::NintendoSwitchJoyConPair => 3,
            _ => 1,
        }
    }

    pub fn get_gamepad_name(&self) -> String {
        self.controller_type.get_name().to_owned()
    }

    pub fn set_rumble(&mut self, state: &SharedGameState, low_freq: u16, hi_freq: u16, ticks: u32) -> GameResult {
        self.play_effect(RumbleEffect::original(low_freq, hi_freq, ticks, state.settings.timing_mode.get_tps() as u32))
    }

    fn play_effect(&mut self, effect: RumbleEffect) -> GameResult {
        if self.rumble.accept(effect, Instant::now()) {
            self.controller.set_rumble(effect.low, effect.high, effect.duration_ms)?;
        }
        Ok(())
    }

    fn stop_rumble(&mut self) -> GameResult {
        if self.rumble.stop() { self.controller.set_rumble(0, 0, 0)?; }
        Ok(())
    }
}

pub struct GamepadContext {
    // Stable session slots keep another player's assignment intact on unplug.
    gamepads: Vec<Option<GamepadData>>,
    input_sequence: u64,
}

impl GamepadContext {
    pub(crate) fn new() -> Self {
        Self { gamepads: Vec::new(), input_sequence: 0 }
    }

    pub(crate) fn automatic_gamepad_index(&self, excluded: Option<u32>) -> Option<u32> {
        self.get_gamepads()
            .filter(|(index, _)| Some(*index as u32) != excluded)
            .max_by_key(|(index, pad)| (pad.last_input, std::cmp::Reverse(*index)))
            .map(|(index, _)| index as u32)
    }

    pub(crate) fn instance_id(&self, index: u32) -> Option<u32> {
        self.get_gamepad_by_index(index as usize).map(|pad| pad.controller.instance_id())
    }

    pub(crate) fn index_for_instance(&self, id: u32) -> Option<u32> {
        self.get_gamepads().find(|(_, pad)| pad.controller.instance_id() == id).map(|(i, _)| i as u32)
    }

    pub(crate) fn input_sequence(&self) -> u64 { self.input_sequence }

    pub(crate) fn last_input(&self, index: u32) -> u64 {
        self.get_gamepad_by_index(index as usize).map_or(0, |pad| pad.last_input)
    }

    pub(crate) fn set_axis_sensitivity(&mut self, index: u32, sensitivity: f64) {
        if let Some(pad) = self.get_gamepad_by_index_mut(index as usize) {
            pad.axis_sensitivity = sensitivity;
        }
    }

    fn get_gamepad(&self, gamepad_id: u32) -> Option<&GamepadData> {
        self.gamepads.iter().flatten().find(|gamepad| gamepad.controller.instance_id() == gamepad_id)
    }

    fn get_gamepad_by_index(&self, gamepad_index: usize) -> Option<&GamepadData> {
        self.gamepads.get(gamepad_index).and_then(Option::as_ref)
    }

    fn get_gamepad_mut(&mut self, gamepad_id: u32) -> Option<&mut GamepadData> {
        self.gamepads.iter_mut().flatten().find(|gamepad| gamepad.controller.instance_id() == gamepad_id)
    }

    fn get_gamepad_by_index_mut(&mut self, gamepad_index: usize) -> Option<&mut GamepadData> {
        self.gamepads.get_mut(gamepad_index).and_then(Option::as_mut)
    }

    pub(crate) fn add_gamepad(&mut self, game_controller: Box<dyn BackendGamepad>, axis_sensitivity: f64) {
        if self.get_gamepad(game_controller.instance_id()).is_some() { return; }
        let data = Some(GamepadData::new(game_controller, axis_sensitivity));
        if let Some(slot) = self.gamepads.iter_mut().find(|slot| slot.is_none()) {
            *slot = data;
        } else {
            self.gamepads.push(data);
        }
    }

    pub(crate) fn remove_gamepad(&mut self, gamepad_id: u32) {
        for slot in &mut self.gamepads {
            if slot.as_ref().is_some_and(|data| data.controller.instance_id() == gamepad_id) {
                if let Some(pad) = slot.as_mut() { let _ = pad.stop_rumble(); }
                *slot = None;
            }
        }
    }

    pub(crate) fn set_gamepad_type(&mut self, gamepad_id: u32, controller_type: GamepadType) {
        if let Some(gamepad) = self.get_gamepad_mut(gamepad_id) {
            gamepad.set_gamepad_type(controller_type);
        }
    }

    pub(crate) fn get_gamepad_sprite_offset(&self, gamepad_index: usize) -> usize {
        if let Some(gamepad) = self.get_gamepad_by_index(gamepad_index) {
            return gamepad.get_gamepad_sprite_offset();
        }

        1
    }

    pub(crate) fn set_button(&mut self, gamepad_id: u32, button: Button, pressed: bool) {
        let activated = pressed && self.get_gamepad(gamepad_id)
            .is_some_and(|pad| !pad.pressed_buttons_set.contains(&button));
        if activated { self.input_sequence += 1; }
        let sequence = self.input_sequence;
        if let Some(gamepad) = self.get_gamepad_mut(gamepad_id) {
            if activated { gamepad.last_input = sequence; }
            if pressed {
                gamepad.pressed_buttons_set.insert(button);
            } else {
                gamepad.pressed_buttons_set.remove(&button);
            }
        }
    }

    pub(crate) fn set_axis_value(&mut self, gamepad_id: u32, axis: Axis, value: f64) {
        let activated = self.get_gamepad(gamepad_id).is_some_and(|pad| {
            let previous = pad.axis_values.get(&axis).copied().unwrap_or(0.0);
            let threshold = pad.axis_sensitivity.max(0.12);
            value.abs() > threshold && (previous.abs() <= threshold || previous.signum() != value.signum())
        });
        if activated { self.input_sequence += 1; }
        let sequence = self.input_sequence;
        if let Some(gamepad) = self.get_gamepad_mut(gamepad_id) {
            if activated { gamepad.last_input = sequence; }
            gamepad.axis_values.insert(axis, value);
        }
    }

    pub(crate) fn is_active(&self, gamepad_index: u32, input_type: &PlayerControllerInputType) -> bool {
        match input_type {
            PlayerControllerInputType::ButtonInput(button) => self.is_button_active(gamepad_index, *button),
            PlayerControllerInputType::EitherButtons(first, second) => {
                self.is_button_active(gamepad_index, *first) || self.is_button_active(gamepad_index, *second)
            }
            PlayerControllerInputType::AxisInput(axis, axis_direction) => {
                self.is_axis_active(gamepad_index, *axis, *axis_direction)
            }
            PlayerControllerInputType::Either(button, axis, axis_direction) => {
                self.is_button_active(gamepad_index, *button)
                    || self.is_axis_active(gamepad_index, *axis, *axis_direction)
            }
        }
    }

    pub(crate) fn is_button_active(&self, gamepad_index: u32, button: Button) -> bool {
        if let Some(gamepad) = self.get_gamepad_by_index(gamepad_index as usize) {
            return gamepad.pressed_buttons_set.contains(&button);
        }

        false
    }

    pub(crate) fn is_axis_active(&self, gamepad_index: u32, axis: Axis, direction: AxisDirection) -> bool {
        if let Some(gamepad) = self.get_gamepad_by_index(gamepad_index as usize) {
            return match axis {
                Axis::LeftX => direction.compare(gamepad.left_x, gamepad.axis_sensitivity),
                Axis::LeftY => direction.compare(gamepad.left_y, gamepad.axis_sensitivity),
                Axis::RightX => direction.compare(gamepad.right_x, gamepad.axis_sensitivity),
                Axis::RightY => direction.compare(gamepad.right_y, gamepad.axis_sensitivity),
                Axis::TriggerLeft => direction.compare(gamepad.trigger_left, 0.0),
                Axis::TriggerRight => direction.compare(gamepad.trigger_right, 0.0),
            };
        }

        false
    }

    pub(crate) fn update_axes(&mut self, gamepad_id: u32) {
        if let Some(gamepad) = self.get_gamepad_mut(gamepad_id) {
            let mut axes = [
                (&mut gamepad.left_x, Axis::LeftX),
                (&mut gamepad.left_y, Axis::LeftY),
                (&mut gamepad.right_x, Axis::RightX),
                (&mut gamepad.right_y, Axis::RightY),
                (&mut gamepad.trigger_left, Axis::TriggerLeft),
                (&mut gamepad.trigger_right, Axis::TriggerRight),
            ];

            for (axis_val, id) in axes.iter_mut() {
                if let Some(axis) = gamepad.axis_values.get(id) {
                    **axis_val = if axis.abs() < 0.12 { 0.0 } else { *axis };
                }
            }
        }
    }

    pub(crate) fn get_gamepads(&self) -> impl Iterator<Item = (usize, &GamepadData)> {
        self.gamepads.iter().enumerate().filter_map(|(index, pad)| pad.as_ref().map(|pad| (index, pad)))
    }

    pub(crate) fn pressed_buttons(&self, gamepad_index: u32) -> HashSet<Button> {
        if let Some(gamepad) = self.get_gamepad_by_index(gamepad_index as usize) {
            return gamepad.pressed_buttons_set.clone();
        }

        HashSet::new()
    }

    pub(crate) fn active_axes(&self, gamepad_index: u32) -> HashMap<Axis, f64> {
        if let Some(gamepad) = self.get_gamepad_by_index(gamepad_index as usize) {
            let mut active_axes = gamepad.axis_values.clone();
            active_axes.retain(|_, v| v.abs() > gamepad.axis_sensitivity);
            return active_axes;
        }

        HashMap::new()
    }

    pub(crate) fn set_rumble(
        &mut self,
        gamepad_index: u32,
        state: &SharedGameState,
        low_freq: u16,
        hi_freq: u16,
        ticks: u32,
    ) -> GameResult {
        if let Some(gamepad) = self.get_gamepad_by_index_mut(gamepad_index as usize) {
            gamepad.set_rumble(state, low_freq, hi_freq, ticks)?;
        }

        Ok(())
    }

    pub(crate) fn set_rumble_all(
        &mut self,
        state: &SharedGameState,
        low_freq: u16,
        hi_freq: u16,
        ticks: u32,
    ) -> GameResult {
        let indices = self.rumble_indices(state);
        for index in indices.into_iter().flatten() {
            self.set_rumble(index, state, low_freq, hi_freq, ticks)?;
        }

        Ok(())
    }

    pub(crate) fn rumble_indices(&self, state: &SharedGameState) -> [Option<u32>; 2] {
        let p1 = if state.settings.player1_rumble { state.settings.player1_gamepad_index(self) } else { None };
        let p2 = if state.player_count == PlayerCount::Two && state.settings.player2_rumble {
            match state.settings.player2_controller_type {
                ControllerType::Gamepad(index) if self.instance_id(index).is_some() && Some(index) != p1 => Some(index),
                _ => None,
            }
        } else { None };
        [p1, p2]
    }

    /// Stop feedback immediately on mode/off/assignment changes. Menus use this too.
    pub(crate) fn reconcile_rumble(&mut self, state: &SharedGameState) -> GameResult {
        let indices = self.rumble_indices(state);
        for (index, pad) in self.gamepads.iter_mut().enumerate() {
            if let Some(pad) = pad {
                let owner = indices.iter().position(|slot| *slot == Some(index as u32));
                let enhanced = match owner {
                    Some(0) => state.settings.player1_rumble_mode == RumbleMode::Enhanced,
                    Some(1) => state.settings.player2_rumble_mode == RumbleMode::Enhanced,
                    _ => false,
                };
                if owner.is_none() || (!enhanced && pad.rumble.enhanced_active()) { pad.stop_rumble()?; }
            }
        }
        Ok(())
    }

    pub(crate) fn play_effect(&mut self, index: u32, effect: RumbleEffect) -> GameResult {
        if let Some(pad) = self.get_gamepad_by_index_mut(index as usize) { pad.play_effect(effect)?; }
        Ok(())
    }

    pub(crate) fn stop_rumble(&mut self) -> GameResult {
        for pad in self.gamepads.iter_mut().flatten() { pad.stop_rumble()?; }
        Ok(())
    }
}

impl Default for GamepadContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod controller_mapping_tests {
    use super::*;

    struct TestGamepad(u32);
    impl BackendGamepad for TestGamepad {
        fn instance_id(&self) -> u32 { self.0 }
        fn set_rumble(&mut self, _: u16, _: u16, _: u32) -> GameResult { Ok(()) }
    }

    #[test]
    fn quake_respects_player_switches_and_does_not_rumble_unassigned_devices() {
        use std::sync::{Arc, Mutex};
        struct RecordingPad(u32, Arc<Mutex<Vec<u32>>>);
        impl BackendGamepad for RecordingPad {
            fn instance_id(&self) -> u32 { self.0 }
            fn set_rumble(&mut self, low: u16, high: u16, _: u32) -> GameResult {
                if low != 0 || high != 0 { self.1.lock().unwrap().push(self.0); }
                Ok(())
            }
        }
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut ctx = Context::new();
        ctx.headless = true;
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut state = SharedGameState::new(&mut ctx).unwrap();
        for id in [10, 20, 30] {
            ctx.gamepad_context.add_gamepad(Box::new(RecordingPad(id, calls.clone())), 0.3);
        }
        state.settings.auto_controller_switching = true;
        state.settings.player1_rumble = false;
        state.settings.player2_rumble = true;
        state.settings.player2_controller_type = crate::game::settings::ControllerType::Gamepad(1);
        state.player_count = crate::game::shared_game_state::PlayerCount::One;
        set_quake_rumble_all(&mut ctx, &state, 10).unwrap();
        assert!(calls.lock().unwrap().is_empty(), "off must suppress story quake too");
        state.player_count = crate::game::shared_game_state::PlayerCount::Two;
        set_quake_rumble_all(&mut ctx, &state, 10).unwrap();
        assert_eq!(*calls.lock().unwrap(), vec![20]);
    }

    #[test]
    fn either_map_button_works_and_releases_across_reconnect() {
        let binding: PlayerControllerInputType =
            serde_json::from_str(r#"{"EitherButtons":["North","Back"]}"#).unwrap();
        let mut pads = GamepadContext::new();
        assert!(!pads.is_active(0, &binding));
        pads.add_gamepad(Box::new(TestGamepad(7)), 0.3);
        for button in [Button::North, Button::Back] {
            pads.set_button(7, button, true);
            assert!(pads.is_active(0, &binding));
            assert!(!pads.is_active(1, &binding));
            pads.set_button(7, button, false);
            assert!(!pads.is_active(0, &binding));
        }
        pads.set_button(7, Button::North, true);
        pads.set_button(7, Button::Back, true);
        pads.set_button(7, Button::North, false);
        assert!(pads.is_active(0, &binding));
        pads.remove_gamepad(7);
        pads.add_gamepad(Box::new(TestGamepad(9)), 0.3);
        assert!(!pads.is_active(0, &binding));
        assert_eq!(binding, serde_json::from_str(&serde_json::to_string(&binding).unwrap()).unwrap());
    }

    #[test]
    fn legacy_single_button_binding_still_round_trips() {
        let binding: PlayerControllerInputType = serde_json::from_str(r#"{"ButtonInput":"North"}"#).unwrap();
        assert_eq!(serde_json::to_string(&binding).unwrap(), r#"{"ButtonInput":"North"}"#);
    }

    #[test]
    fn auto_selects_real_activity_but_not_release_or_axis_drift() {
        let mut pads = GamepadContext::new();
        assert_eq!(pads.automatic_gamepad_index(None), None);
        pads.add_gamepad(Box::new(TestGamepad(41)), 0.3);
        pads.add_gamepad(Box::new(TestGamepad(73)), 0.3);
        assert_eq!(pads.automatic_gamepad_index(None), Some(0));
        pads.set_axis_value(73, Axis::LeftX, 0.2);
        assert_eq!(pads.automatic_gamepad_index(None), Some(0));
        pads.set_button(73, Button::South, true);
        assert_eq!(pads.automatic_gamepad_index(None), Some(1));
        assert_eq!(pads.automatic_gamepad_index(Some(1)), Some(0));
        pads.set_button(41, Button::North, false);
        assert_eq!(pads.automatic_gamepad_index(None), Some(1));
        pads.set_axis_value(41, Axis::LeftX, 0.8);
        assert_eq!(pads.automatic_gamepad_index(None), Some(0));
        pads.set_button(73, Button::North, true);
        pads.set_axis_value(41, Axis::LeftX, 0.9);
        assert_eq!(pads.automatic_gamepad_index(None), Some(1));
        pads.remove_gamepad(73);
        assert_eq!(pads.automatic_gamepad_index(None), Some(0));
        assert_eq!(pads.automatic_gamepad_index(Some(0)), None);
    }

    #[test]
    fn disconnect_keeps_other_players_slot_and_reuses_vacancy() {
        let mut pads = GamepadContext::new();
        pads.add_gamepad(Box::new(TestGamepad(41)), 0.3);
        pads.add_gamepad(Box::new(TestGamepad(73)), 0.3);
        pads.set_button(73, Button::South, true);
        pads.remove_gamepad(41);
        assert!(pads.is_button_active(1, Button::South));
        assert!(!pads.is_button_active(0, Button::South));
        pads.add_gamepad(Box::new(TestGamepad(90)), 0.3);
        pads.set_button(90, Button::North, true);
        assert!(pads.is_button_active(0, Button::North));
        assert!(!pads.is_button_active(1, Button::North));
    }

    #[test]
    fn sensitivity_updates_follow_reused_slots_not_instance_ids() {
        let mut pads = GamepadContext::new();
        pads.add_gamepad(Box::new(TestGamepad(41)), 0.3);
        pads.add_gamepad(Box::new(TestGamepad(73)), 0.3);
        pads.set_button(73, Button::South, true);
        pads.remove_gamepad(41);
        pads.add_gamepad(Box::new(TestGamepad(90)), 0.3);
        let slot = pads.index_for_instance(90).unwrap();
        assert_eq!(slot, 0);
        pads.set_axis_sensitivity(slot, 0.6);
        pads.set_axis_value(90, Axis::LeftX, 0.4);
        assert_eq!(pads.automatic_gamepad_index(None), Some(1));
        pads.set_axis_value(90, Axis::LeftX, 0.8);
        assert_eq!(pads.automatic_gamepad_index(None), Some(0));
    }
}

pub fn add_gamepad(context: &mut Context, game_controller: Box<dyn BackendGamepad>, axis_sensitivity: f64) {
    context.gamepad_context.add_gamepad(game_controller, axis_sensitivity);
}

pub fn remove_gamepad(context: &mut Context, gamepad_id: u32) {
    context.gamepad_context.remove_gamepad(gamepad_id);
}

pub fn set_gamepad_type(context: &mut Context, gamepad_id: u32, controller_type: GamepadType) {
    context.gamepad_context.set_gamepad_type(gamepad_id, controller_type);
}

pub fn get_gamepad_sprite_offset(context: &Context, gamepad_index: usize) -> usize {
    context.gamepad_context.get_gamepad_sprite_offset(gamepad_index)
}

pub fn is_active(ctx: &Context, gamepad_index: u32, input_type: &PlayerControllerInputType) -> bool {
    ctx.gamepad_context.is_active(gamepad_index, input_type)
}

pub fn is_button_active(ctx: &Context, gamepad_index: u32, button: Button) -> bool {
    ctx.gamepad_context.is_button_active(gamepad_index, button)
}

pub fn is_axis_active(ctx: &Context, gamepad_index: u32, axis: Axis, direction: AxisDirection) -> bool {
    ctx.gamepad_context.is_axis_active(gamepad_index, axis, direction)
}

pub fn get_gamepads(ctx: &Context) -> impl Iterator<Item = (usize, &GamepadData)> {
    ctx.gamepad_context.get_gamepads()
}

pub fn pressed_buttons(ctx: &Context, gamepad_index: u32) -> HashSet<Button> {
    ctx.gamepad_context.pressed_buttons(gamepad_index)
}

pub fn active_axes(ctx: &Context, gamepad_index: u32) -> HashMap<Axis, f64> {
    ctx.gamepad_context.active_axes(gamepad_index)
}

pub fn set_rumble(
    ctx: &mut Context,
    state: &SharedGameState,
    gamepad_index: u32,
    low_freq: u16,
    hi_freq: u16,
    ticks: u32,
) -> GameResult {
    ctx.gamepad_context.set_rumble(gamepad_index, state, low_freq, hi_freq, ticks)
}

pub fn set_rumble_all(
    ctx: &mut Context,
    state: &SharedGameState,
    low_freq: u16,
    hi_freq: u16,
    ticks: u32,
) -> GameResult {
    ctx.gamepad_context.set_rumble_all(state, low_freq, hi_freq, ticks)
}

pub fn set_quake_rumble(ctx: &mut Context, state: &SharedGameState, gamepad_index: u32, ticks: u32) -> GameResult {
    set_rumble(ctx, state, gamepad_index, QUAKE_RUMBLE_LOW_FREQ, QUAKE_RUMBLE_HI_FREQ, ticks)
}

pub fn set_quake_rumble_all(ctx: &mut Context, state: &SharedGameState, ticks: u32) -> GameResult {
    set_rumble_all(ctx, state, QUAKE_RUMBLE_LOW_FREQ, QUAKE_RUMBLE_LOW_FREQ, ticks)
}

pub fn set_super_quake_rumble(
    ctx: &mut Context,
    state: &SharedGameState,
    gamepad_index: u32,
    ticks: u32,
) -> GameResult {
    set_rumble(ctx, state, gamepad_index, SUPER_QUAKE_RUMBLE_LOW_FREQ, SUPER_QUAKE_RUMBLE_HI_FREQ, ticks)
}

pub fn set_super_quake_rumble_all(ctx: &mut Context, state: &SharedGameState, ticks: u32) -> GameResult {
    set_rumble_all(ctx, state, SUPER_QUAKE_RUMBLE_LOW_FREQ, SUPER_QUAKE_RUMBLE_LOW_FREQ, ticks)
}
