"""Independently verify ten release artifacts, package identities and original resources."""
import argparse
import ctypes
from ctypes import wintypes
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import struct
import subprocess
from datetime import datetime
import zipfile

import install_english_resources as english
import install_japanese_resources as japanese
import patch_distribution_notice as notice
from package_windows_with_game import HANS_HASH, MACHINES, pe_machine

CERTIFICATE = 'ea385afc82e19824eea8a8868ed365df03e9886bbba33e47e1e04b43ec65cb67'
ARCHIVES = {'dou_sc102.zip': HANS_HASH, 'cavestoryen.zip': english.SHA256, 'dou_1006.zip': japanese.SHA256}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def inspect_zip(package):
    names = package.namelist()
    require(package.testzip() is None, 'ZIP CRC failure')
    require(len(names) == len(set(names)), 'Duplicate ZIP members')
    for name in names:
        path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts and '\\' not in name and ':' not in name, 'Unsafe ZIP member: ' + name)
        leaf = path.name.lower()
        require(not leaf.startswith('profile') and leaf not in ('settings.json', 'config.dat'), 'Player state included: ' + name)
    return names


def no_game(names):
    require(not any(name.lower().endswith(('.tsc', '.pxm', '.pxe', 'doukutsu.exe')) or name.startswith('assets/game/') for name in names), 'Game data leaked into base package')


def windows_version_resource(executable):
    """Read both string and fixed versions using Windows' resource parser."""
    api = ctypes.WinDLL('version', use_last_error=True)
    api.GetFileVersionInfoSizeW.argtypes = [wintypes.LPCWSTR, ctypes.POINTER(wintypes.DWORD)]
    api.GetFileVersionInfoSizeW.restype = wintypes.DWORD
    api.GetFileVersionInfoW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p]
    api.GetFileVersionInfoW.restype = wintypes.BOOL
    api.VerQueryValueW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(wintypes.UINT)]
    api.VerQueryValueW.restype = wintypes.BOOL
    ignored = wintypes.DWORD()
    size = api.GetFileVersionInfoSizeW(str(executable), ctypes.byref(ignored))
    require(size > 0, 'Missing Windows version resource')
    buffer = ctypes.create_string_buffer(size)
    require(api.GetFileVersionInfoW(str(executable), 0, size, buffer), 'Cannot read Windows version resource')

    def query(key):
        pointer, length = ctypes.c_void_p(), wintypes.UINT()
        require(api.VerQueryValueW(buffer, key, ctypes.byref(pointer), ctypes.byref(length)), 'Missing version field: ' + key)
        return pointer, length.value

    pointer, length = query('\\')
    require(length >= 52, 'Truncated fixed version info')
    fixed = struct.unpack('<13I', ctypes.string_at(pointer, 52))
    require(fixed[0] == 0xfeef04bd, 'Invalid fixed version signature')
    pointer, length = query('\\VarFileInfo\\Translation')
    require(length >= 4 and length % 4 == 0, 'Invalid resource translations')
    translations = ctypes.string_at(pointer, length)
    strings = []
    for offset in range(0, length, 4):
        language, codepage = struct.unpack_from('<HH', translations, offset)
        entry = {}
        for field in ('FileVersion', 'ProductVersion'):
            pointer, count = query(f'\\StringFileInfo\\{language:04x}{codepage:04x}\\{field}')
            entry[field] = ctypes.wstring_at(pointer, count).rstrip('\0')
        strings.append(entry)
    return {'strings': strings, 'file_numeric': (fixed[2] << 32) | fixed[3],
            'product_numeric': (fixed[4] << 32) | fixed[5]}


def verify_windows_version(resource, date):
    parsed = datetime.strptime(date, '%Y%m%d')
    expected = (parsed.year << 48) | (parsed.month << 32) | (parsed.day << 16)
    require(resource['strings'] and all(entry['FileVersion'] == date and entry['ProductVersion'] == date
                                      for entry in resource['strings']), 'Windows string versions differ from build date')
    require(resource['file_numeric'] == expected and resource['product_numeric'] == expected,
            'Windows numeric versions differ from build date')


def android_identity(apk, date):
    sdk = Path(os.environ.get('ANDROID_HOME', str(Path.home() / 'AppData/Local/Android/Sdk'))) / 'build-tools/35.0.1'
    env = os.environ.copy()
    if not env.get('JAVA_HOME'):
        env['JAVA_HOME'] = 'C:/Program Files/Android/Android Studio/jbr'
    signature = subprocess.run([str(sdk / 'apksigner.bat'), 'verify', '--verbose', '--print-certs', str(apk)], capture_output=True, text=True, encoding='utf-8', errors='replace', check=True, env=env)
    require(CERTIFICATE in signature.stdout.lower(), 'Unexpected APK signing certificate')
    badging = subprocess.run([str(sdk / 'aapt.exe'), 'dump', 'badging', str(apk)], capture_output=True, text=True, encoding='utf-8', errors='replace', check=True).stdout
    require("package: name='io.github.cavestory_rs'" in badging, 'Release application ID changed')
    require("versionName='" + date + "'" in badging, 'APK versionName differs from batch date')
    require("versionCode='" + str(int(date)) + "'" in badging, 'APK versionCode differs from batch date')
    require('application-debuggable' not in badging, 'Debug APK in release delivery')


def verify(batch):
    info = json.loads((batch / 'build-info.json').read_text(encoding='utf-8-sig'))
    date = info['build_date']
    datetime.strptime(date, '%Y%m%d')
    require(re.fullmatch(r'\d{8}', date) is not None and info['version'] == date and info['configuration'] == 'Release', 'Invalid release batch version')
    records = []
    def record(path, **extra):
        records.append(dict(path=path.relative_to(batch).as_posix(), bytes=path.stat().st_size, sha256=digest(path.read_bytes()), build_date=date, **extra))
    for abi, elf_class, machine in [('arm64-v8a', 2, 183), ('armeabi-v7a', 1, 40)]:
        base_libs = None
        for kind in ('base', 'game'):
            apk, = (batch / 'android' / abi / kind).glob('*.apk')
            expected = 'CaveStory-rs_android_' + date + '_' + abi + ('_game' if kind == 'game' else '') + '.apk'
            require(apk.name == expected, 'APK naming mismatch')
            android_identity(apk, date)
            with zipfile.ZipFile(apk) as package:
                names = inspect_zip(package)
                licenses = Path(__file__).resolve().parents[1] / 'drsandroid/app/src/main/assets/licenses'
                for license_name in ('Shizuku-API-LICENSE.txt', 'CaveStory-rs-LICENSE.txt', 'SDL2-LICENSE.txt'):
                    entry = 'assets/licenses/' + license_name
                    require(entry in names, 'Missing Android license: ' + license_name)
                    require(package.read(entry) == (licenses / license_name).read_bytes(), 'Android license differs: ' + license_name)
                libs = [name for name in names if name.startswith('lib/') and name.endswith('.so')]
                require(len(libs) == 3, 'Unexpected native library count')
                native = {}
                for name in libs:
                    require(name.startswith('lib/' + abi + '/'), 'Wrong ABI split')
                    payload = package.read(name)
                    require(payload[:4] == b'\x7fELF' and payload[4] == elf_class and struct.unpack_from('<H', payload, 18)[0] == machine, 'Wrong ELF architecture')
                    native[name] = digest(payload)
                    if name.endswith('/libdrsandroid.so'):
                        require(date.encode('ascii') in payload, 'Android engine compiled build date is missing')
                require(any(name.endswith('/libdrsandroid.so') for name in libs), 'Missing Android engine library')
                if kind == 'base':
                    no_game(names)
                    base_libs = native
                else:
                    require(native == base_libs, 'Native libraries differ between base/game')
                    sources = json.loads(package.read('assets/game/source-manifest.json'))
                    require(set(sources) == set(ARCHIVES), 'Bundled source manifest mismatch')
                    for name, expected_hash in ARCHIVES.items():
                        content = package.read('assets/game/' + name)
                        require(digest(content) == expected_hash == sources[name]['sha256'], 'Bundled archive hash mismatch')
                        require(len(content) == sources[name]['bytes'], 'Bundled source size mismatch')
                for language, font in [('english', 'fonts/chinese-12.fnt'), ('japanese', 'fonts/japanese/chinese-12.fnt')]:
                    locale = json.loads(package.read('assets/' + language + '-locale.json'))
                    require(locale['encoding'] == 'shift_jis' and locale['font'] == font, 'Wrong language encoding/font')
                for overlay_name in ('chinese-overlay.zip', 'japanese-fonts.zip'):
                    with zipfile.ZipFile(io.BytesIO(package.read('assets/' + overlay_name))) as overlay:
                        overlay_names = inspect_zip(overlay)
                        no_game(overlay_names)
                        require(any(name.endswith('licenses/OFL.txt') for name in overlay_names), 'Missing font license')
            record(apk, elf_verified=True, signature_verified=True, configuration='Release', package_kind=kind)
    for arch, machine in MACHINES.items():
        base_payload = None
        for kind in ('engine', 'with-game'):
            archive, = (batch / 'windows' / arch / kind).glob('*.zip')
            expected = 'CaveStory-rs_windows_' + date + '_' + arch + ('_game' if kind == 'with-game' else '')
            require(archive.stem == expected, 'Windows artifact naming mismatch')
            with zipfile.ZipFile(archive) as package:
                names = inspect_zip(package)
                root = expected + '/'
                payload = package.read(root + 'CaveStory-rs.exe')
                require(pe_machine(payload) == machine, 'Wrong engine PE machine')
                # The displayed version is compiled in as a literal by build.rs.
                require(date.encode() in payload, 'Engine build date is missing')
                if kind == 'engine':
                    no_game(names)
                    base_payload = payload
                    require(payload == (archive.parent / root / 'CaveStory-rs.exe').read_bytes(), 'Staging EXE differs')
                    verify_windows_version(windows_version_resource(archive.parent / root / 'CaveStory-rs.exe'), date)
                else:
                    require(payload == base_payload, 'Engine differs between base/game ZIP')
                    staged_executable = archive.parent / 'staging' / root / 'CaveStory-rs.exe'
                    require(payload == staged_executable.read_bytes(), 'Game staging EXE differs')
                    verify_windows_version(windows_version_resource(staged_executable), date)
                    manifest = json.loads((archive.parent / 'source-manifest.json').read_text(encoding='utf-8'))
                    require(manifest['build_date'] == date and manifest['arch'] == arch and manifest['engine_sha256'] == digest(payload), 'Source manifest engine/date mismatch')
                    require(manifest['chinese_sha256'] == HANS_HASH and manifest['english_sha256'] == english.SHA256 and manifest['japanese_sha256'] == japanese.SHA256, 'Source hash mismatch')
                    require(manifest['personal_saves_included'] is False, 'Source manifest includes player data')
                    require(any(item['name'] == 'vcruntime140.dll' for item in manifest['runtime_files'].values()), 'Missing runtime')
                    for runtime in manifest['runtime_files'].values():
                        content = package.read(root + runtime['name'])
                        require(pe_machine(content) == machine and digest(content) == runtime['sha256'], 'Runtime hash/architecture mismatch')
                    for name, expected_hash in manifest['original_chinese_files'].items():
                        if name.replace('\\', '/') in manifest.get('distribution_script_patches', {}):
                            expected_hash = manifest['distribution_script_patches'][name.replace('\\', '/')]['after_sha256']
                        require(digest(package.read(root + name)) == expected_hash, 'Chinese original mismatch: ' + name)
                    for code, language, expected_hash in [('en', 'english', english.SHA256), ('jp', 'japanese', japanese.SHA256)]:
                        prefix = root + 'data/' + code + '/'
                        installed = json.loads(package.read(prefix + language + '-install.json'))
                        require(installed['archive_sha256'] == expected_hash, 'Language manifest source hash mismatch')
                        for name, original_hash in installed['original_files'].items():
                            patch_path = 'data/' + code + '/' + name.replace('\\', '/')
                            if patch_path in manifest.get('distribution_script_patches', {}):
                                original_hash = manifest['distribution_script_patches'][patch_path]['after_sha256']
                            require(digest(package.read(prefix + name)) == original_hash, 'Original language resource mismatch: ' + name)
                        require(prefix + 'Readme.txt' in names and prefix + 'Stage/Start.tsc' in names, 'Incomplete language data')
                    require(root + 'data/fonts/japanese/chinese-12.fnt' in names, 'Missing Japanese font')
                    patches = manifest.get('distribution_script_patches', {})
                    require(len(patches) == 3, 'Expected three distribution script patches')
                    for path, patch_record in patches.items():
                        original = package.read(root + path.rsplit('/Stage/', 1)[0] + '/.distribution-notice/Barr.tsc.original')
                        require(digest(original) == notice.LANGUAGES[patch_record['language']][1] == patch_record['before_sha256'], 'Script original backup mismatch')
                        patched = package.read(root + path)
                        require(notice.patch_bytes(original, patch_record['language']) == patched and digest(patched) == patch_record['after_sha256'], 'Unexpected modified story data')
            record(archive, pe_machine=hex(machine), package_kind=kind, no_player_state=True)
    require(len(records) == 10, 'Expected ten delivery artifacts')
    (batch / 'verified-delivery.json').write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    (batch / 'SHA256SUMS').write_text(''.join(record['sha256'] + '  ' + record['path'] + '\n' for record in records), encoding='utf-8')
    print('Verified four Release APKs and six Windows ZIPs across five architectures')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('batch', type=Path)
    verify(parser.parse_args().batch.resolve())
