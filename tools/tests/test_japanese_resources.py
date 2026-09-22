import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_japanese_resources as japanese
import build_chinese_font as font


class JapaneseResourcesTests(unittest.TestCase):
    def test_japanese_font_uses_pinned_regional_glyphs(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = font.build(japanese.RUNS / 'downloads' / font.URL.rsplit('/', 1)[1],
                                Path(temporary) / 'font', japanese=True)
            self.assertEqual(result['locale'], 'jp')
            self.assertIn('fusion-pixel-12px-monospaced-ja.bdf', result['source_members_sha256'])

    def test_pinned_install_preserves_base_english_and_saves(self):
        archive = japanese.RUNS / 'downloads/dou_1006.zip'
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'chinese-install.json').write_text('{}')
            (root / 'en').mkdir()
            (root / 'en/Head.tsc').write_bytes(b'English unchanged')
            (root / 'Profile.dat').write_bytes(b'player save')
            before = {p.relative_to(root): p.read_bytes() for p in root.rglob('*') if p.is_file()}
            result = japanese.install(root, archive)
            for path, payload in before.items():
                self.assertEqual((root / path).read_bytes(), payload)
            metadata = json.loads((result / 'japanese-install.json').read_text())
            for path, digest in metadata['original_files'].items():
                self.assertEqual(hashlib.sha256((result / path).read_bytes()).hexdigest(), digest)
            self.assertTrue((result / 'Stage/Start.tsc').is_file())
            self.assertTrue((result / 'Readme.txt').is_file())
            self.assertTrue((result / 'fonts/japanese/chinese-12.fnt').is_file())
            self.assertFalse((result / 'Config.dat').exists())
            self.assertEqual(japanese.japanese_locale()['encoding'], 'shift_jis')
            self.assertEqual(japanese.japanese_locale()['font'], 'fonts/japanese/chinese-12.fnt')

    def test_unmanaged_and_existing_custom_directory_are_protected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(ValueError, 'managed'):
                japanese.install(root, root / 'absent.zip')
            (root / 'chinese-install.json').write_text('{}')
            (root / 'jp').mkdir()
            (root / 'jp/custom.txt').write_text('keep')
            with self.assertRaises(FileExistsError):
                japanese.install(root, root / 'absent.zip')
            self.assertEqual((root / 'jp/custom.txt').read_text(), 'keep')

    def test_corrupt_download_is_rejected_without_publication(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'chinese-install.json').write_text('{}')
            archive = root / 'bad.zip'
            archive.write_bytes(b'corrupt')
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                japanese.install(root, archive)
            self.assertFalse((root / 'jp').exists())
