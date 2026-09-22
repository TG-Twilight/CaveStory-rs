import importlib.util
from pathlib import Path
import struct
import unittest
import json

SPEC = importlib.util.spec_from_file_location(
    'check_chinese_font', Path(__file__).resolve().parents[1] / 'check_chinese_font.py')
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


class CoverageTests(unittest.TestCase):
    def test_remaining_image_labels_are_optional_and_validated(self):
        english = audit.flatten(json.loads(audit.ENGLISH.read_text(encoding='utf-8')))
        for key, text in {'game.hud.level': '级', 'game.hud.max': '满',
                          'game.stage_select': '传送', 'game.boss': '首领'}.items():
            with self.subTest(key=key):
                audit.validate_locale(dict(english, **{key: text}), english)
                for invalid in (' \t ', '{value}'):
                    with self.assertRaises(ValueError):
                        audit.validate_locale(dict(english, **{key: invalid}), english)

    def test_hud_and_caret_labels_are_optional_and_validated(self):
        english = audit.flatten(json.loads(audit.ENGLISH.read_text(encoding='utf-8')))
        labels = {'game.hud.air': '氧气', 'game.caret.level_up': '升级',
                  'game.caret.level_down': '降级', 'game.caret.empty': '空弹'}
        for key, text in labels.items():
            with self.subTest(key=key):
                audit.validate_locale(dict(english, **{key: text}), english)
                for invalid in (' ', '{value}'):
                    with self.assertRaises(ValueError):
                        audit.validate_locale(dict(english, **{key: invalid}), english)
        with self.assertRaises(ValueError):
            audit.validate_locale(dict(english, **{'game.caret.levelup': '升级'}), english)

    def test_inventory_labels_are_optional_locale_extensions(self):
        english = audit.flatten(json.loads(audit.ENGLISH.read_text(encoding='utf-8')))
        chinese = dict(english, encoding='gbk', stage_encoding='gbk')
        self.assertTrue(hasattr(audit, 'validate_locale'), 'Locale extension validator is missing')
        audit.validate_locale(chinese, english)
        chinese.update({'game.inventory.weapons': '武器', 'game.inventory.items': '物品'})
        audit.validate_locale(chinese, english)

    def test_inventory_extensions_do_not_hide_locale_errors(self):
        english = audit.flatten(json.loads(audit.ENGLISH.read_text(encoding='utf-8')))
        valid = dict(english, encoding='gbk', stage_encoding='gbk')
        valid.update({'game.inventory.weapons': '武器', 'game.inventory.items': '物品'})
        self.assertTrue(hasattr(audit, 'validate_locale'), 'Locale extension validator is missing')
        missing = dict(valid)
        del missing['common.back']
        typo = dict(valid)
        typo['game.inventory.wepon'] = '武器'
        placeholder = dict(valid)
        placeholder['game.cutscene_skip'] = '跳过剧情'
        empty = dict(valid)
        empty['game.inventory.items'] = ' '
        for invalid in (missing, typo, placeholder, empty):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                audit.validate_locale(invalid, english)

    def test_ignores_bmf_notdef_like_engine(self):
        # The built-in font includes id=0xffffffff; Rust char::from_u32 skips it.
        records = b''.join(struct.pack('<IHHHHhhhBB', code, 0, 0, 1, 1, 0, 0, 1, 0, 15)
                           for code in (0xffffffff, 0xd800, 65, 20013))
        data = b'BMF\x03' + struct.pack('<BI', 4, len(records)) + records
        self.assertTrue(hasattr(audit, 'font_characters'), 'Engine-compatible character reader is missing')
        self.assertEqual(audit.font_characters(data), {'A', '中'})


if __name__ == '__main__':
    unittest.main()
