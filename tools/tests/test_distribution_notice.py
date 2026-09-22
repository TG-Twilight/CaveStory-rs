import hashlib
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import patch_distribution_notice as notice

RUNS = Path(__file__).resolve().parents[3] / 'CaveStory-rs-runs'
SOURCES = [('zh-Hans', 'dou_sc102.zip', 'Doukutsu'),
           ('en', 'cavestoryen.zip', 'CaveStory'), ('jp', 'dou_1006.zip', 'doukutsu')]


class DistributionNoticeTest(unittest.TestCase):
    def original(self, archive, root):
        with zipfile.ZipFile(RUNS / 'downloads' / archive) as source:
            return source.read(root + '/data/Stage/Barr.tsc')

    def test_patch_preserves_original_story_and_returns_to_it_in_all_languages(self):
        for language, archive, root in SOURCES:
            with self.subTest(language=language):
                original = self.original(archive, root)
                patched = notice.patch_bytes(original, language)
                self.assertNotEqual(original, patched, 'The initial Balrog encounter must receive the dialogue')
                plain = notice.decrypt(patched)
                before = notice.decrypt(original)
                self.assertEqual(plain.count(b'<PSH9500'), 1)
                self.assertEqual(plain.split(b'\n#9500')[0].replace(b'<PSH9500', b'').rstrip(), before.rstrip())
                self.assertIn(b'<GHP', plain)
                self.assertIn(b'<POP', plain)
                self.assertEqual(notice.patch_bytes(patched, language), patched)

    def test_unknown_or_modified_scripts_are_preserved(self):
        for language, archive, root in SOURCES:
            original = self.original(archive, root)
            modified = original[:-1] + bytes([original[-1] ^ 1])
            self.assertEqual(notice.patch_bytes(modified, language), modified)

    def test_previous_release_upgrades_with_original_backup_and_preserves_custom_edits(self):
        for language, archive, root in SOURCES:
            with self.subTest(language=language), tempfile.TemporaryDirectory(dir=RUNS / 'maintenance') as temp:
                original = self.original(archive, root)
                old_fragment = (Path(__file__).parent / 'fixtures/distribution-v1' / (language + '.txt')).read_text(encoding='utf-8').encode(notice.LANGUAGES[language][0])
                old = notice.encrypt(notice.decrypt(original).replace(b'<FAC0005<MSG', b'<PSH9500<FAC0005<MSG') + b'\n' + old_fragment)
                data = Path(temp)
                script = data / 'Stage/Barr.tsc'
                script.parent.mkdir()
                script.write_bytes(old)
                backup = data / '.distribution-notice/Barr.tsc.original'
                backup.parent.mkdir()
                backup.write_bytes(original)
                expected = notice.patch_bytes(original, language)
                self.assertNotEqual(old, expected)
                notice.install_language(data, language)
                self.assertEqual(script.read_bytes(), expected, 'Previously shipped dialogue must upgrade')
                self.assertEqual(backup.read_bytes(), original)
                self.assertIsNone(notice.install_language(data, language))
                custom = old[:-1] + bytes([old[-1] ^ 1])
                script.write_bytes(custom)
                notice.install_language(data, language)
                self.assertEqual(script.read_bytes(), custom)

    def test_dialogue_starts_after_balrog_complaint_before_original_challenge(self):
        for language, archive, root in SOURCES:
            plain = notice.decrypt(notice.patch_bytes(self.original(archive, root), language))
            self.assertLess(plain.index(b'<FAC0005<MSG'), plain.index(b'<PSH9500'))
            self.assertLess(plain.index(b'<PSH9500'), plain.index(b'<YNJ1001'))

    def test_install_backs_up_original_and_never_touches_saves_or_custom_script(self):
        with tempfile.TemporaryDirectory(dir=RUNS / 'maintenance') as temp:
            data = Path(temp)
            script = data / 'Stage/Barr.tsc'
            script.parent.mkdir()
            original = self.original('dou_sc102.zip', 'Doukutsu')
            script.write_bytes(original)
            (data / 'Profile.dat').write_bytes(b'player save')
            record = notice.install_language(data, 'zh-Hans')
            self.assertNotEqual(script.read_bytes(), original)
            self.assertEqual((data / '.distribution-notice/Barr.tsc.original').read_bytes(), original)
            self.assertEqual(record['before_sha256'], hashlib.sha256(original).hexdigest())
            patched = script.read_bytes()
            notice.install_language(data, 'zh-Hans')
            self.assertEqual(script.read_bytes(), patched)
            self.assertEqual((data / 'Profile.dat').read_bytes(), b'player save')
            script.write_bytes(b'custom story')
            notice.install_language(data, 'zh-Hans')
            self.assertEqual(script.read_bytes(), b'custom story')

    def test_interrupted_backup_never_publishes_partial_original_or_blocks_retry(self):
        with tempfile.TemporaryDirectory(dir=RUNS / 'maintenance') as temp:
            data = Path(temp)
            script = data / 'Stage/Barr.tsc'
            script.parent.mkdir()
            original = self.original('dou_sc102.zip', 'Doukutsu')
            script.write_bytes(original)
            with patch.object(notice.os, 'fsync', side_effect=OSError('simulated full disk')):
                with self.assertRaises(OSError):
                    notice.install_language(data, 'zh-Hans')
            self.assertFalse((data / '.distribution-notice/Barr.tsc.original').exists())
            self.assertEqual(script.read_bytes(), original)
            notice.install_language(data, 'zh-Hans')
            self.assertTrue(script.read_bytes() != original)


if __name__ == '__main__':
    unittest.main()
