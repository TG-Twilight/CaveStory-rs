import sys
from pathlib import Path
import tempfile
import unittest
import shutil

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install_english_resources as english


class EnglishResourcesTests(unittest.TestCase):
    def test_verified_legacy_copy_without_marker_can_upgrade(self):
        legacy = english.RUNS / 'runtime-copies/android-hans-20260913-v3/payload/data'
        if not legacy.is_dir():
            self.skipTest('External legacy verification copy not available')
        with tempfile.TemporaryDirectory() as temporary:
            data = Path(temporary) / 'data'
            shutil.copytree(legacy, data)
            self.assertFalse((data / 'chinese-install.json').exists())
            original = (data / 'Head.tsc').read_bytes()
            english.install(data, english.RUNS / 'downloads/cavestoryen.zip')
            self.assertEqual((data / 'Head.tsc').read_bytes(), original)
            self.assertFalse((data / 'chinese-install.json').exists())
            shutil.rmtree(data / 'en')
            (data / 'Head.tsc').write_bytes(b'custom story')
            with self.assertRaisesRegex(ValueError, 'verified original'):
                english.install(data, english.RUNS / 'downloads/cavestoryen.zip')

    def test_corrupt_archive_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / 'bad.zip'
            archive.write_bytes(b'not the verified archive')
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                english.archive_bytes(archive)

    def test_unmanaged_and_existing_directories_are_protected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(ValueError, 'managed'):
                english.install(root, root / 'absent.zip')
            (root / 'chinese-install.json').write_text('{}')
            (root / 'en').mkdir()
            (root / 'en/custom.txt').write_text('keep')
            with self.assertRaises(FileExistsError):
                english.install(root, root / 'absent.zip')
            self.assertEqual((root / 'en/custom.txt').read_text(), 'keep')

    def test_pinned_archive_preserves_base_and_saves(self):
        archive = english.RUNS / 'downloads/cavestoryen.zip'
        if not archive.is_file():
            self.skipTest('Pinned external game archive not available')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'chinese-install.json').write_text('{}')
            (root / 'Head.tsc').write_bytes(b'Chinese base unchanged')
            (root / 'Profile.dat').write_bytes(b'player progress')
            before = {p.name: p.read_bytes() for p in root.iterdir()}
            installed = english.install(root, archive)
            for name, payload in before.items():
                self.assertEqual((root / name).read_bytes(), payload)
            self.assertTrue((installed / 'Stage/Start.tsc').is_file())
            self.assertTrue((installed / 'Readme.txt').is_file())
            self.assertFalse((installed / 'Config.dat').exists())
            self.assertFalse((installed / 'Profile.dat').exists())
            self.assertNotEqual((installed / 'Head.tsc').read_bytes(), before['Head.tsc'])
            self.assertEqual(english.english_locale()['encoding'], 'shift_jis')
