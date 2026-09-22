"""Packaging boundary checks; no toolchain or signing key needed."""
import io
import struct
import unittest
from copy import deepcopy
import zipfile

import package_windows_with_game as packaging
import verify_local_delivery as delivery


class DeliveryTests(unittest.TestCase):
    def test_windows_versions_reject_stale_string_and_numeric_fields(self):
        numeric = (2026 << 48) | (9 << 32) | (14 << 16)
        resource = {'strings': [{'FileVersion': '20260914', 'ProductVersion': '20260914'}],
                    'file_numeric': numeric, 'product_numeric': numeric}
        delivery.verify_windows_version(resource, '20260914')
        for field in ('FileVersion', 'ProductVersion'):
            wrong = deepcopy(resource)
            wrong['strings'][0][field] = '1.0.0'
            with self.subTest(field=field), self.assertRaises(ValueError):
                delivery.verify_windows_version(wrong, '20260914')
        for field in ('file_numeric', 'product_numeric'):
            wrong = deepcopy(resource)
            wrong[field] -= 65536
            with self.subTest(field=field), self.assertRaises(ValueError):
                delivery.verify_windows_version(wrong, '20260914')

    def test_expected_architectures(self):
        self.assertEqual(set(packaging.MACHINES), {'x86_64', 'x86_32', 'arm64'})
        for machine in packaging.MACHINES.values():
            data = bytearray(256)
            data[:2] = b'MZ'
            struct.pack_into('<I', data, 60, 128)
            data[128:132] = b'PE\0\0'
            struct.pack_into('<H', data, 132, machine)
            self.assertEqual(packaging.pe_machine(data), machine)
            data[128] = 0
            with self.assertRaises(ValueError):
                packaging.pe_machine(data)

    def test_base_rejects_bundled_archives_and_loose_game(self):
        for path in ['assets/game/dou_sc102.zip', 'data/Stage/Start.tsc', 'data/Stage/Start.pxm', 'Doukutsu.exe']:
            with self.subTest(path=path), self.assertRaises(ValueError):
                delivery.no_game([path])
        delivery.no_game(['data/fonts/chinese-12.fnt', 'assets/japanese-fonts.zip'])

    def test_rejects_player_state_and_unsafe_paths(self):
        for path in ['user/Profile.dat', 'user/settings.json', '../outside', '/absolute', 'C:/outside']:
            memory = io.BytesIO()
            with zipfile.ZipFile(memory, 'w') as package:
                package.writestr(path, b'test')
            with zipfile.ZipFile(memory) as package, self.subTest(path=path), self.assertRaises(ValueError):
                delivery.inspect_zip(package)

    def test_preserves_original_archives(self):
        for name, expected in delivery.ARCHIVES.items():
            archive = packaging.english.RUNS / 'downloads' / name
            self.assertEqual(packaging.sha(archive), expected)

    def test_matching_runtime_architectures(self):
        for arch, machine in packaging.MACHINES.items():
            files = packaging.runtime_files(arch)
            self.assertIn('vcruntime140.dll', [path.name for path in files])
            self.assertTrue(all(packaging.pe_machine(path.read_bytes()) == machine for path in files))


if __name__ == '__main__':
    unittest.main()
