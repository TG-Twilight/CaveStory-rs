//! Runtime hints only: input availability and saved device assignments remain independent.
use crate::common::Rect;
use crate::engine_constants::EngineConstants;
use crate::framework::context::Context;
use crate::framework::gamepad::{Axis, Button, PlayerControllerInputType};
use crate::framework::keyboard::ScanCode;
use crate::game::settings::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputSource { Keyboard, Gamepad(u32), Touch }

#[derive(Default)]
pub struct PromptHistory { sources: Vec<InputSource> }
impl PromptHistory {
    fn record(&mut self, source: InputSource) {
        self.sources.retain(|old| *old != source);
        self.sources.push(source);
    }
    pub(crate) fn remove_gamepad(&mut self, id: u32) {
        self.sources.retain(|source| *source != InputSource::Gamepad(id));
    }
}

pub fn source(ctx: &Context, settings: &Settings) -> InputSource {
    let pad = settings.player1_gamepad_index(&ctx.gamepad_context)
        .and_then(|index| ctx.gamepad_context.instance_id(index));
    ctx.prompt_history.sources.iter().rev().copied().find(|source| match source {
        InputSource::Keyboard => true,
        InputSource::Touch => settings.touch_controls,
        InputSource::Gamepad(id) => {
            if settings.auto_controller_switching {
                ctx.gamepad_context.index_for_instance(*id).is_some_and(|index|
                    settings.player2_controller_type != crate::game::settings::ControllerType::Gamepad(index))
            } else {
                Some(*id) == pad
            }
        },
    }).unwrap_or(if settings.touch_controls { InputSource::Touch } else { InputSource::Keyboard })
}

fn bindings(settings: &Settings) -> [PlayerControllerInputType; 14] {
    let map = &settings.player1_controller_button_map;
    [map.left, map.up, map.right, map.down, map.prev_weapon, map.next_weapon,
        map.jump, map.shoot, map.skip, map.inventory, map.map, map.strafe, map.menu_ok, map.menu_back]
}

impl Context {
    pub(crate) fn prompt_key(&mut self, settings: &Settings, key: ScanCode, pressed: bool) {
        let map = &settings.player1_key_map;
        if pressed && !self.keyboard_context.is_key_pressed(key)
            && [map.left, map.up, map.right, map.down, map.prev_weapon, map.next_weapon,
                map.jump, map.shoot, map.skip, map.inventory, map.map, map.strafe,
                map.menu_ok, map.menu_back, ScanCode::Escape, ScanCode::Return].contains(&key) {
            self.prompt_history.record(InputSource::Keyboard);
        }
        self.keyboard_context.set_key(key, pressed);
    }
    fn prompt_gamepad_activity(&mut self, settings: &Settings, id: u32, before: u64) {
        if let Some(index) = self.gamepad_context.index_for_instance(id) {
            if self.gamepad_context.last_input(index) != before
                && settings.player1_gamepad_index(&self.gamepad_context) == Some(index) {
                self.prompt_history.record(InputSource::Gamepad(id));
            }
        }
    }
    pub(crate) fn prompt_button(&mut self, settings: &Settings, id: u32, button: Button, pressed: bool) {
        let before = self.gamepad_context.index_for_instance(id).map_or(0, |i| self.gamepad_context.last_input(i));
        self.gamepad_context.set_button(id, button, pressed);
        let mapped = button == Button::Start || bindings(settings).iter().any(|binding| match *binding {
            PlayerControllerInputType::ButtonInput(b) | PlayerControllerInputType::Either(b, _, _) => b == button,
            PlayerControllerInputType::EitherButtons(a, b) => a == button || b == button,
            _ => false,
        });
        if mapped { self.prompt_gamepad_activity(settings, id, before); }
    }
    pub(crate) fn prompt_axis(&mut self, settings: &Settings, id: u32, axis: Axis, value: f64) {
        let before = self.gamepad_context.index_for_instance(id).map_or(0, |i| self.gamepad_context.last_input(i));
        self.gamepad_context.set_axis_value(id, axis, value);
        let mapped = bindings(settings).iter().any(|binding| match *binding {
            PlayerControllerInputType::AxisInput(a, direction) | PlayerControllerInputType::Either(_, a, direction) =>
                a == axis && direction.compare(value, settings.player1_controller_axis_sensitivity),
            _ => false,
        });
        if mapped { self.prompt_gamepad_activity(settings, id, before); }
    }
    pub(crate) fn prompt_touch(&mut self, enabled: bool) {
        if enabled { self.prompt_history.record(InputSource::Touch); }
    }
}

/// Hit-test the same gameplay areas as TouchPlayerController. Menu touch
/// handling has no gameplay layout and accepts a new touch anywhere on screen.
pub(crate) fn touch_is_effective(
    kind: crate::input::touch_controls::TouchControlType,
    canvas: (f32, f32), insets: (f32, f32, f32, f32), point: (f64, f64),
) -> bool {
    use crate::input::touch_controls::TouchControlType;
    let (x, y) = point;
    let (w, h) = (canvas.0 as f64, canvas.1 as f64);
    let (left, top, right, bottom) = (4.0 + insets.0 as f64, 4.0 + insets.1 as f64,
        4.0 + insets.2 as f64, 4.0 + insets.3 as f64);
    let inside = |l, t, width, height| x > l && x < l + width && y > t && y < t + height;
    match kind {
        TouchControlType::None => inside(0.0, 0.0, w, h),
        TouchControlType::Dialog => inside(0.0, h / 2.0, w, h / 2.0)
            || inside(w - 48.0 - right, top, 48.0, 48.0),
        TouchControlType::Controls => {
            let dpad = inside(left, h - bottom - 144.0, 144.0, 144.0)
                && !inside(left + 48.0, h - bottom - 96.0, 48.0, 48.0);
            dpad || inside(w - 48.0 - right, bottom, 48.0, 48.0)
                || inside(w - 48.0 - right, h - 52.0 - bottom, 48.0, 48.0)
                || inside(w - 48.0 - right, h - 104.0 - bottom, 48.0, 48.0)
                || inside(0.0, 0.0, 40.0, 40.0)
        }
    }
}

pub struct BindingPrompt {
    pub key: String,
    pub symbols: Vec<(char, Rect<u16>)>,
}

pub fn skip_prompt(ctx: &Context, settings: &Settings, constants: &EngineConstants) -> BindingPrompt {
    match source(ctx, settings) {
        InputSource::Keyboard => BindingPrompt { key: format!("{:?}", settings.player1_key_map.skip), symbols: vec![] },
        InputSource::Touch => BindingPrompt { key: ">>".into(), symbols: vec![] },
        InputSource::Gamepad(id) => {
            let index = ctx.gamepad_context.index_for_instance(id).unwrap();
            let offset = ctx.gamepad_context.get_gamepad_sprite_offset(index as usize);
            binding_prompt(settings.player1_controller_button_map.skip, offset, constants)
        }
    }
}

fn binding_prompt(binding: PlayerControllerInputType, offset: usize, constants: &EngineConstants) -> BindingPrompt {
    use PlayerControllerInputType::*;
    let bindings = match binding {
        EitherButtons(a, b) => vec![ButtonInput(a), ButtonInput(b)],
        Either(button, axis, direction) => vec![ButtonInput(button), AxisInput(axis, direction)],
        binding => vec![binding],
    };
    let mut key = String::new();
    let mut symbols = Vec::new();
    for (i, binding) in bindings.iter().enumerate() {
        if i > 0 { key.push('/'); }
        let symbol = if i == 0 { '\u{e000}' } else { '\u{e001}' };
        let rect = binding.get_rect(offset, constants);
        if rect.width() == 0 { key.push_str("Guide"); } else {
            key.push(symbol);
            symbols.push((symbol, rect));
        }
        if let AxisInput(_, direction) = binding {
            use crate::framework::gamepad::AxisDirection::*;
            key.push_str(match direction { Left => "<", Right => ">", Up => "^", Down => "v", _ => "" });
        }
    }
    BindingPrompt { key, symbols }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::{backend::BackendGamepad, error::GameResult, gamepad::Button};
    struct Pad(u32);
    impl BackendGamepad for Pad {
        fn instance_id(&self) -> u32 { self.0 }
        fn set_rumble(&mut self, _: u16, _: u16, _: u32) -> GameResult { Ok(()) }
    }
    #[test]
    fn automatic_keyboard_configuration_uses_active_pad_prompt() {
        let mut ctx = Context::new();
        let settings = Settings::default();
        ctx.gamepad_context.add_gamepad(Box::new(Pad(42)), 0.3);
        ctx.prompt_button(&settings, 42, Button::West, true);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
    }
    #[test]
    fn ordered_switches_ignore_repeat_release_noise_and_player_two() {
        use crate::framework::gamepad::Axis;
        use crate::game::settings::ControllerType;
        let mut ctx = Context::new();
        let mut settings = Settings::default();
        settings.touch_controls = true;
        settings.player2_controller_type = ControllerType::Gamepad(1);
        ctx.gamepad_context.add_gamepad(Box::new(Pad(42)), 0.3);
        ctx.gamepad_context.add_gamepad(Box::new(Pad(43)), 0.3);
        assert_eq!(source(&ctx, &settings), InputSource::Touch);
        ctx.prompt_key(&settings, ScanCode::Q, true);
        ctx.prompt_button(&settings, 42, Button::West, true);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
        ctx.prompt_key(&settings, ScanCode::Q, true);
        ctx.prompt_key(&settings, ScanCode::Q, false);
        ctx.prompt_key(&settings, ScanCode::F12, true);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
        ctx.prompt_key(&settings, ScanCode::Q, true);
        ctx.prompt_axis(&settings, 42, Axis::LeftX, 0.1);
        ctx.prompt_button(&settings, 43, Button::South, true);
        assert_eq!(source(&ctx, &settings), InputSource::Keyboard);
        ctx.prompt_axis(&settings, 42, Axis::LeftX, 0.8);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
        ctx.prompt_touch(true);
        ctx.prompt_axis(&settings, 42, Axis::LeftX, 0.9);
        assert_eq!(source(&ctx, &settings), InputSource::Touch);
        ctx.prompt_axis(&settings, 42, Axis::LeftX, -0.8);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
        ctx.gamepad_context.remove_gamepad(42);
        ctx.prompt_history.remove_gamepad(42);
        assert_eq!(source(&ctx, &settings), InputSource::Touch);
        ctx.gamepad_context.add_gamepad(Box::new(Pad(99)), 0.3);
        assert_eq!(source(&ctx, &settings), InputSource::Touch);
        assert!(settings.touch_controls);
    }
    #[test]
    fn hints_use_custom_binding_alternatives_and_hardware_row() {
        use crate::framework::gamepad::{AxisDirection, GamepadType};
        let mut ctx = Context::new();
        let mut settings = Settings::default();
        let constants = EngineConstants::defaults();
        ctx.gamepad_context.add_gamepad(Box::new(Pad(42)), 0.3);
        ctx.gamepad_context.set_gamepad_type(42, GamepadType::XboxOne);
        ctx.prompt_button(&settings, 42, Button::West, true);
        let xbox = skip_prompt(&ctx, &settings, &constants);
        assert_eq!(xbox.symbols.len(), 1);
        settings.player1_controller_button_map = crate::game::settings::psp_controller_button_map();
        assert_eq!(skip_prompt(&ctx, &settings, &constants).symbols[0].1.left, xbox.symbols[0].1.left);

        settings.player1_controller_button_map.skip = PlayerControllerInputType::EitherButtons(Button::North, Button::Back);
        let two = skip_prompt(&ctx, &settings, &constants);
        assert_eq!(two.symbols.len(), 2);
        assert!(two.key.contains('/'));
        assert_ne!(two.symbols[0].1.left, two.symbols[1].1.left);
        settings.player1_controller_button_map.skip = PlayerControllerInputType::Either(Button::West, Axis::RightX, AxisDirection::Left);
        let axis = skip_prompt(&ctx, &settings, &constants);
        assert_eq!(axis.symbols.len(), 2);
        assert!(axis.key.ends_with('<'));
        ctx.gamepad_context.set_gamepad_type(42, GamepadType::PS4);
        assert_ne!(skip_prompt(&ctx, &settings, &constants).symbols[0].1.left, axis.symbols[0].1.left);
        ctx.prompt_key(&settings, ScanCode::Q, true);
        settings.player1_key_map.skip = ScanCode::Space;
        assert_eq!(skip_prompt(&ctx, &settings, &constants).key, "Space");
    }

    #[test]
    fn manual_assignment_and_disconnected_pad_fall_back_to_keyboard() {
        use crate::game::settings::ControllerType;
        let mut ctx = Context::new();
        let mut settings = Settings::default();
        settings.auto_controller_switching = false;
        settings.player1_controller_type = ControllerType::Gamepad(0);
        ctx.gamepad_context.add_gamepad(Box::new(Pad(42)), 0.3);
        ctx.gamepad_context.add_gamepad(Box::new(Pad(43)), 0.3);
        ctx.prompt_button(&settings, 43, Button::West, true);
        assert_eq!(source(&ctx, &settings), InputSource::Keyboard);
        ctx.prompt_button(&settings, 42, Button::West, true);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
        ctx.gamepad_context.remove_gamepad(42);
        assert_eq!(source(&ctx, &settings), InputSource::Keyboard);
    }

    #[test]
    fn touch_uses_dialogue_and_gameplay_hit_areas() {
        use crate::input::touch_controls::TouchControlType::*;
        let canvas = (320.0, 240.0);
        let insets = (0.0, 0.0, 0.0, 0.0);
        assert!(!touch_is_effective(Dialog, canvas, insets, (160.0, 60.0)));
        assert!(touch_is_effective(Dialog, canvas, insets, (290.0, 25.0)));
        assert!(touch_is_effective(Dialog, canvas, insets, (160.0, 180.0)));
        assert!(!touch_is_effective(Controls, canvas, insets, (170.0, 80.0)));
        assert!(!touch_is_effective(Controls, canvas, insets, (75.0, 160.0)));
        for point in [(20.0, 160.0), (75.0, 110.0), (120.0, 210.0), (290.0, 200.0), (290.0, 145.0)] {
            assert!(touch_is_effective(Controls, canvas, insets, point));
        }
    }

    #[test]
    fn unbound_gamepad_button_does_not_replace_keyboard_prompt() {
        let mut ctx = Context::new();
        let settings = Settings::default();
        ctx.gamepad_context.add_gamepad(Box::new(Pad(42)), 0.3);
        ctx.prompt_button(&settings, 42, Button::Guide, true);
        assert_eq!(source(&ctx, &settings), InputSource::Keyboard);
    }

    #[test]
    fn unbound_second_pad_does_not_invalidate_last_effective_pad_hint() {
        let mut ctx = Context::new();
        let settings = Settings::default();
        ctx.gamepad_context.add_gamepad(Box::new(Pad(42)), 0.3);
        ctx.gamepad_context.add_gamepad(Box::new(Pad(43)), 0.3);
        ctx.prompt_button(&settings, 42, Button::West, true);
        ctx.prompt_button(&settings, 43, Button::Guide, true);
        assert_eq!(source(&ctx, &settings), InputSource::Gamepad(42));
        // Raw activity stays available to rebind/device selection.
        assert_eq!(settings.player1_gamepad_index(&ctx.gamepad_context), Some(1));
    }

}
