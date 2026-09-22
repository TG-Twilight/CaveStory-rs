use super::*;
use crate::common::Color;
use crate::data::builtin_fs::BuiltinFS;
use crate::game::map::Map;
use crate::game::shared_game_state::TileSize;
use crate::game::stage::{Background, BackgroundType, NpcType, Stage, StageData, Tileset};

#[test]
fn project_link_opcode_compiles_without_consuming_the_following_command() {
    let script = TextScript::compile(b"#0001\n<GHP<WAI0010<END", true, TextScriptEncoding::UTF8);
    assert!(script.is_ok(), "project link must be a supported zero-operand TSC command");
}

// Exercise the compiler and VM without a window, audio device, game assets or writable saves.
fn run_message(text: &str, face: u16, fast: bool) -> (SharedGameState, Vec<String>) {
    run_message_choices(text, face, fast, None)
}

fn run_message_choices(text: &str, face: u16, fast: bool, mut answers: Option<Vec<bool>>) -> (SharedGameState, Vec<String>) {
    let mut ctx = Context::new();
    ctx.headless = true;
    ctx.filesystem.mount_vfs(Box::new(BuiltinFS::new()));
    let mut state = SharedGameState::new(&mut ctx).unwrap();
    let data = StageData {
        name: String::new(),
        name_jp: String::new(),
        map: String::new(),
        boss_no: 0,
        tileset: Tileset::new("0"),
        pxpack_data: None,
        background: Background::new("0"),
        background_type: BackgroundType::Black,
        background_color: Color::from_rgb(0, 0, 0),
        npc1: NpcType::new("0"),
        npc2: NpcType::new("0"),
    };
    state.stages.push(data.clone());
    let stage = Stage {
        map: Map { width: 0, height: 0, tiles: vec![], attrib: [0; 0x100], tile_size: TileSize::Tile16x16 },
        data,
    };
    let mut scene = GameScene::from_stage(&mut state, &mut ctx, stage, 0).unwrap();
    // Built-in BMF at native scale: 12px ideographs, 6px ASCII and a 16px inline icon.
    state.font =
        crate::graphics::bmfont::BMFont::load(&vec!["/".into()], "/builtin/builtin_font.fnt", &mut ctx, 1.0).unwrap();
    state.textscript_vm.substitution_rect_map[0] = ('=', Rect::new_size(0, 0, 16, 12));
    let script = format!("#0001\n<MSG<FAC{face:04}{text}<WAI9999");
    state.textscript_vm
        .set_scene_script(TextScript::compile(script.as_bytes(), true, TextScriptEncoding::UTF8).unwrap());
    state.textscript_vm.start_script(1);
    state.textscript_vm.suspend = false;
    state.textscript_vm.flags.set_perma_fast(fast);
    let mut scrolled = Vec::new();
    let mut was_scrolling = false;
    for _ in 0..10000 {
        TextScriptVM::run(&mut state, &mut scene, &mut ctx).unwrap();
        if let Some(choices) = answers.as_mut() {
            match state.textscript_vm.state {
                TextScriptExecutionState::WaitInput(event, ip, _) => {
                    state.textscript_vm.state = TextScriptExecutionState::Running(event, ip);
                }
                TextScriptExecutionState::WaitConfirmation(event, ip, no_event, _, _) => {
                    assert!(!choices.is_empty(), "unexpected additional question");
                    state.textscript_vm.state = if choices.remove(0) {
                        TextScriptExecutionState::Running(event, ip)
                    } else {
                        TextScriptExecutionState::Running(no_event, 0)
                    };
                }
                _ => {}
            }
        }
        let vm = &state.textscript_vm;
        let scrolling = matches!(vm.state, TextScriptExecutionState::MsgNewLine(..));
        if scrolling && !was_scrolling {
            scrolled.push(vm.line_1.iter().collect());
        }
        was_scrolling = scrolling;
        if matches!(vm.state, TextScriptExecutionState::WaitTicks(_, _, 9999)) {
            if let Some(choices) = &answers { assert!(choices.is_empty(), "a required question was skipped"); }
            return (state, scrolled);
        }
    }
    panic!("message did not finish");
}

#[test]
fn distribution_dialogue_all_answers_return_to_original_event() {
    for fragment in [include_str!("../../../../../res/distribution/zh-Hans.txt"),
                     include_str!("../../../../../res/distribution/en.txt"),
                     include_str!("../../../../../res/distribution/jp.txt")] {
        // Exercise actual TSC branches and POP, without filesystem or native popups.
        for choices in [vec![true], vec![false, true, false], vec![false, false, true, false],
                        vec![false, false, false, false], vec![false, true, true], vec![false, false, true, true]] {
            let source = format!("<PSH9500<MSG<FAC0005ORIGINAL<WAI9999\n{fragment}");
            let (state, _) = run_message_choices(&source, 5, true, Some(choices));
            assert_eq!(lines(&state)[0], "ORIGINAL");
            assert!(state.textscript_vm.stack.is_empty());
            assert_eq!(state.textscript_vm.face, 5);
        }
    }
}

#[test]
fn resetting_a_script_clears_pending_external_link_confirmation() {
    let mut vm = TextScriptVM::new();
    vm.confirmed_project_position = Some((9504, 10));
    vm.start_script(9504);
    assert!(vm.confirmed_project_position.is_none());
}

fn lines(state: &SharedGameState) -> Vec<String> {
    [&state.textscript_vm.line_1, &state.textscript_vm.line_2, &state.textscript_vm.line_3]
        .iter()
        .map(|line| line.iter().collect())
        .collect()
}

#[test]
fn dialogue_wraps_before_crossing_box_padding() {
    let (state, _) = run_message(&"甲".repeat(20), 0, false);
    assert_eq!(lines(&state), vec!["甲".repeat(18), "甲".repeat(2), String::new()]);
}

#[test]
fn dialogue_reserves_portrait_space_and_measures_mixed_symbols() {
    let (state, _) = run_message(&format!("{}A1=乙。", "甲".repeat(11)), 1, false);
    assert_eq!(lines(&state), vec![format!("{}A1=", "甲".repeat(11)), "乙。".into(), String::new()]);
}

#[test]
fn dialogue_exact_fit_then_crlf_only_advances_once() {
    let (state, _) = run_message(&format!("{}\r\n乙", "甲".repeat(18)), 0, false);
    assert_eq!(lines(&state), vec!["甲".repeat(18), "乙".into(), String::new()]);
}

#[test]
fn dialogue_preserves_last_character_when_wrapping_scrolls() {
    for fast in [false, true] {
        let (state, _) = run_message(&format!("甲\n乙\n{}丁", "丙".repeat(18)), 0, fast);
        assert_eq!(lines(&state), vec!["乙".into(), "丙".repeat(18), "丁".into()]);
    }
}

#[test]
fn dialogue_manual_scroll_preserves_single_remaining_character() {
    let (state, _) = run_message("甲\n乙\n丙\n丁", 0, false);
    assert_eq!(lines(&state), vec!["乙", "丙", "丁"]);
}

#[test]
fn dialogue_terminal_newline_resumes_the_following_command() {
    let (state, _) = run_message("甲\n乙\n丙\n", 0, false);
    assert_eq!(lines(&state), vec!["乙", "丙", ""]);
}

#[test]
fn dialogue_wraps_across_a_command_without_clearing_text() {
    let (state, _) = run_message(&format!("{}<WAI0001乙", "甲".repeat(18)), 0, false);
    assert_eq!(lines(&state), vec!["甲".repeat(18), "乙".into(), String::new()]);
}

#[test]
fn dialogue_continuous_scroll_keeps_all_characters_in_order() {
    let text = format!("{}{}{}{}", "甲".repeat(18), "乙".repeat(18), "丙".repeat(18), "丁".repeat(18));
    let (state, scrolled) = run_message(&text, 0, false);
    assert_eq!(scrolled.concat() + &lines(&state).concat(), text);
    assert_eq!(lines(&state), vec!["乙".repeat(18), "丙".repeat(18), "丁".repeat(18)]);
}
