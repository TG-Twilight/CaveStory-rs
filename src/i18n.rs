use std::collections::HashMap;

use crate::engine_constants::DataType;
use crate::framework::context::Context;
use crate::framework::filesystem;
use crate::game::scripting::tsc::text_script::TextScriptEncoding;
use crate::game::shared_game_state::FontData;

pub const MANAGED_LANGUAGE_PACKS: [(&str, &str); 2] = [
    ("en", "english-install.json"), ("jp", "japanese-install.json"),
];

#[derive(Debug, Clone)]
pub struct Locale {
    pub code: String,
    pub name: String,
    pub font: FontData,
    pub encoding: Option<TextScriptEncoding>,
    pub stage_encoding: Option<TextScriptEncoding>,
    strings: HashMap<String, String>,

    // Properties of the translation data "root"
    pub is_present: bool,
    pub is_complete: bool, // if the translation is complete, it's present
    pub data_type: Option<DataType>,
}

#[cfg(test)]
mod tests {
    use super::Locale;

    #[test]
    fn japanese_menu_has_all_english_keys_and_control_placeholder() {
        let english = Locale::flatten(&serde_json::from_str(include_str!("data/builtin/builtin_data/locale/en.json")).unwrap());
        let japanese = Locale::flatten(&serde_json::from_str(include_str!("data/builtin/builtin_data/locale/jp.json")).unwrap());
        for key in english.keys() {
            assert!(japanese.contains_key(key), "Japanese menu missing {key}");
        }
        assert!(japanese["menus.controls_menu.rebind_confirm_menu.title"].contains("{control}"));
        assert_ne!(japanese["menus.options_menu.graphics_menu.vsync_mode.uncapped"], english["menus.options_menu.graphics_menu.vsync_mode.uncapped"]);
    }

    #[test]
    fn incomplete_japanese_pack_falls_back_and_keeps_saved_language() {
        use crate::framework::{context::Context, vfs::PhysicalFS};
        let root = std::env::temp_dir().join(format!("cavestory-japanese-incomplete-{}", std::process::id()));
        std::fs::create_dir_all(root.join("jp")).unwrap();
        std::fs::write(root.join("jp/japanese-install.json"), b"{}").unwrap();
        let mut ctx = Context::new();
        ctx.filesystem.mount_vfs(Box::new(PhysicalFS::new(&root, true)));
        ctx.filesystem.mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
        let mut constants = crate::engine_constants::EngineConstants::defaults();
        constants.active_root.support_locales = true;
        let mut settings = crate::game::settings::Settings::default();
        settings.locale = "jp".into();
        constants.rebuild_path_list(None, crate::game::shared_game_state::Season::current(), &settings);
        constants.load_locales(&mut ctx).unwrap();
        constants.rebuild_path_list(None, crate::game::shared_game_state::Season::current(), &settings);
        assert!(!constants.base_paths.contains(&"/jp/".into()));
        assert!(!constants.locales.iter().find(|locale| locale.code == "jp").unwrap().is_present);
        assert_eq!(settings.locale, "jp");
        drop(ctx);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn japanese_pack_metadata_overrides_legacy_gbk_but_not_mod_metadata() {
        use crate::framework::{context::Context, vfs::PhysicalFS};
        let root = std::env::temp_dir().join(format!("cavestory-japanese-locale-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        for dir in ["locale", "jp/locale", "mod/locale"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("jp/japanese-install.json"), b"{}").unwrap();
        for (path, name, encoding) in [("locale/jp.json", "Legacy", "gbk"),
            ("jp/locale/jp.json", "Japanese", "shift_jis"), ("mod/locale/jp.json", "Custom", "utf-8")] {
            std::fs::write(root.join(path), format!(r#"{{"name":"{name}","font":"font.fnt","font_scale":"1.0","encoding":"{encoding}"}}"#)).unwrap();
        }
        let mut ctx = Context::new();
        ctx.filesystem.mount_vfs(Box::new(PhysicalFS::new(&root, true)));
        assert_eq!(Locale::new(&mut ctx, &vec!["/".into()], "jp").name, "Legacy");
        std::fs::write(root.join("jp/stage.sect"), vec![0; 19000]).unwrap();
        assert_eq!(Locale::new(&mut ctx, &vec!["/".into()], "jp").name, "Japanese");
        assert_eq!(Locale::new(&mut ctx, &vec!["/mod/".into(), "/".into()], "jp").name, "Custom");
        std::fs::remove_file(root.join("jp/locale/jp.json")).unwrap();
        assert_eq!(Locale::new(&mut ctx, &vec!["/".into()], "jp").name, "Legacy");
        drop(ctx);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn english_pack_metadata_requires_complete_pack_and_respects_custom_root() {
        use crate::framework::{context::Context, vfs::PhysicalFS};
        let root = std::env::temp_dir().join(format!("cavestory-english-locale-{}", std::process::id()));
        for dir in ["locale", "en/locale", "mod/locale"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("chinese-install.json"), b"{}").unwrap();
        std::fs::write(root.join("en/english-install.json"), b"{}").unwrap();
        for (path, name, encoding) in [("locale/en.json", "Legacy", "gbk"),
            ("en/locale/en.json", "English", "shift_jis"), ("mod/locale/en.json", "Custom", "utf-8")] {
            std::fs::write(root.join(path), format!(r#"{{"name":"{name}","font":"font.fnt","font_scale":"1.0","encoding":"{encoding}"}}"#)).unwrap();
        }
        let mut ctx = Context::new();
        ctx.filesystem.mount_vfs(Box::new(PhysicalFS::new(&root, true)));
        assert_eq!(Locale::new(&mut ctx, &vec!["/".into()], "en").name, "Legacy");
        std::fs::write(root.join("en/stage.sect"), vec![0; 19000]).unwrap();
        assert_eq!(Locale::new(&mut ctx, &vec!["/".into()], "en").name, "English");
        assert_eq!(Locale::new(&mut ctx, &vec!["/mod/".into(), "/".into()], "en").name, "Custom");
        std::fs::remove_file(root.join("en/locale/en.json")).unwrap();
        assert_eq!(Locale::new(&mut ctx, &vec!["/".into()], "en").name, "Legacy");
        drop(ctx);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_chinese_rumble_labels_are_normalized_without_changing_custom_text() {
        let mut locale = Locale::default();
        locale.code = "zh-Hans".into();
        locale.strings.insert("menus.controls_menu.rumble".into(), "手柄震动：".into());
        assert_eq!(locale.t("menus.controls_menu.rumble"), "手柄振动：");
        locale.strings.insert("menus.controls_menu.rumble".into(), "我的触觉设置".into());
        assert_eq!(locale.t("menus.controls_menu.rumble"), "我的触觉设置");
        locale.strings.insert("story".into(), "手柄震动：".into());
        assert_eq!(locale.t("story"), "手柄震动：");
    }

    #[test]
    fn legacy_language_packs_get_touch_options_without_overwriting_custom_labels() {
        for code in ["zh-Hans", "en", "jp"] {
            let mut locale = Locale::default();
            locale.code = code.into();
            for key in ["menus.options_menu.controls_menu.auto_hide_touch_controls",
                "menus.options_menu.controls_menu.touch_controls_required"] {
                assert_ne!(locale.t(key), key, "missing fallback for {code}");
                locale.strings.insert(key.into(), "Custom touch option".into());
                assert_eq!(locale.t(key), "Custom touch option");
            }
        }
    }
}

impl Default for Locale {
    fn default() -> Self {
        Locale {
            code: "en".to_owned(),
            name: "English".to_owned(),
            font: FontData { path: String::new(), scale: 1.0, space_offset: 0.0 },
            encoding: None,
            stage_encoding: None,
            strings: HashMap::new(),

            is_present: false,
            is_complete: false,
            data_type: None,
        }
    }
}

impl Locale {
    pub fn new(ctx: &mut Context, base_paths: &Vec<String>, code: &str) -> Locale {
        // Legacy Chinese overlays also override other languages with GBK.
        // Complete independent packs supply their own metadata, below mods.
        let language_pack = MANAGED_LANGUAGE_PACKS.iter().any(|(pack_code, marker)|
            code == *pack_code && filesystem::exists(ctx, format!("/{code}/{marker}")))
            && filesystem::exists(ctx, format!("/{code}/stage.sect"))
            && filesystem::exists(ctx, format!("/{code}/locale/{code}.json"))
            && base_paths.iter().any(|path| path == "/")
            && base_paths.iter().take_while(|path| path.as_str() != "/")
                .all(|path| matches!(path.as_str(), "/en/" | "/zh-Hans/" | "/jp/"));
        let file = if language_pack {
            filesystem::open(ctx, format!("/{code}/locale/{code}.json"))
        } else {
            filesystem::open_find(ctx, base_paths, &format!("locale/{code}.json"))
        }.unwrap();
        let json: serde_json::Value = serde_json::from_reader(file).unwrap();

        let strings = Locale::flatten(&json);

        let name = strings["name"].clone();

        let font_name = strings["font"].clone();
        let font_scale = strings["font_scale"].parse::<f32>().unwrap_or(1.0);
        let font = FontData::new(font_name, font_scale, 0.0);

        let encoding = if let Some(enc) = strings.get("encoding").clone() {
            Some(TextScriptEncoding::from(enc.as_str()))
        } else {
            None
        };
        let stage_encoding = if let Some(enc) = strings.get("stage_encoding").clone() {
            Some(TextScriptEncoding::from(enc.as_str()))
        } else {
            None
        };

        // This info will be set by the caller
        let is_present = false;
        let is_complete = false;
        let data_type = None;

        Locale { code: code.to_string(), name, font, encoding, stage_encoding, strings, is_present, is_complete, data_type }
    }

    fn flatten(json: &serde_json::Value) -> HashMap<String, String> {
        let mut strings = HashMap::new();

        for (key, value) in json.as_object().unwrap() {
            match value {
                serde_json::Value::String(string) => {
                    strings.insert(key.to_owned(), string.to_owned());
                }
                serde_json::Value::Object(_) => {
                    let substrings = Locale::flatten(value);

                    for (sub_key, sub_value) in substrings.iter() {
                        strings.insert(format!("{}.{}", key, sub_key), sub_value.to_owned());
                    }
                }
                _ => {}
            }
        }

        strings
    }

    /// if the key does not exists, return the origin key instead
    pub fn t<'a: 'b, 'b>(&'a self, key: &'b str) -> &'b str {
        self.t_optional(key).unwrap_or(key)
    }

    /// Look up an optional translation, allowing callers to retain a non-text fallback.
    pub fn t_optional(&self, key: &str) -> Option<&str> {
        // Installed language packs are preserved across APK upgrades. Supply new
        // engine labels when those older packs do not yet contain them.
        let text = self.strings.get(key).map(String::as_str).or_else(|| match key {
            "menus.options_menu.controls_menu.auto_hide_touch_controls" => Some(match self.code.as_str() {
                "zh-Hans" => "自动隐藏触屏按键（10秒）：",
                "jp" => "タッチボタン自動非表示（10秒）: ",
                _ => "Auto-hide touch buttons (10s):",
            }),
            "menus.options_menu.controls_menu.touch_controls_required" => Some(match self.code.as_str() {
                "zh-Hans" => "触屏按键：无外设时自动开启",
                "jp" => "タッチボタン：外部機器なしで表示",
                _ => "Touch buttons: on (no devices)",
            }),
            _ => None,
        })?;
        // Older downloaded overlays remain user-owned on disk. Normalize only
        // our known menu labels; preserve custom translations and story data.
        Some(match (self.code.as_str(), key, text) {
            ("zh-Hans", "menus.controls_menu.rumble", "手柄震动：") => "手柄振动：",
            ("zh-Hans", "menus.controls_menu.rumble_mode", "震动模式：") => "振动模式：",
            ("zh-Hans", "menus.options_menu.graphics_menu.screen_shake.entry", "画面震动强度：") => "画面振动强度：",
            _ => text,
        })
    }

    pub fn tt(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut string = self.t(key).to_owned();

        for (key, value) in args.iter() {
            string = string.replace(&format!("{{{}}}", key), &value);
        }

        string
    }

    pub fn set_font(&mut self, font: FontData) {
        self.font = font;
    }
}
