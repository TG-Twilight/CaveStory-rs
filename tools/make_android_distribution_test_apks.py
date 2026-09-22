"""Create isolated signed/re-signed APKs to test distribution notices without user data.

Only fixed .rt/.ru test package IDs are allowed. No device is modified by this tool.
"""
import argparse
import os
from pathlib import Path
import struct
import subprocess
import zipfile

ORIGINAL = 'io.github.cavestory_rs'


def clone_manifest(data, package):
    data = bytearray(data)
    position = 8
    replaced = 0
    while position < len(data):
        kind, header, size = struct.unpack_from('<HHI', data, position)
        if kind == 1:
            count, _, flags, start, _ = struct.unpack_from('<IIIII', data, position + 8)
            for i in range(count):
                offset = position + start + struct.unpack_from('<I', data, position + header + 4 * i)[0]
                if flags & 0x100:
                    def length(at):
                        if data[at] & 128:
                            return ((data[at] & 127) << 8) | data[at + 1], at + 2
                        return data[at], at + 1
                    _, offset = length(offset)
                    nbytes, offset = length(offset)
                    encoding = 'utf-8'
                else:
                    units = struct.unpack_from('<H', data, offset)[0]
                    if units & 0x8000:
                        units = ((units & 0x7fff) << 16) | struct.unpack_from('<H', data, offset + 2)[0]
                        offset += 2
                    offset += 2
                    nbytes = units * 2
                    encoding = 'utf-16-le'
                value = bytes(data[offset:offset + nbytes]).decode(encoding)
                if value == ORIGINAL or value in (ORIGINAL + '.documents', ORIGINAL + '.shizuku',
                        ORIGINAL + '.androidx-startup', ORIGINAL + '.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION') or value.startswith(ORIGINAL + '.permission.'):
                    replacement = value.replace(ORIGINAL, package, 1).encode(encoding)
                    assert len(replacement) == nbytes
                    data[offset:offset + nbytes] = replacement
                    replaced += 1
        if size < 8:
            raise ValueError('Invalid binary manifest')
        position += size
    if replaced < 2:
        raise ValueError('Package identity or provider was not replaced')
    return bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apk', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    runs = Path(__file__).resolve().parents[2] / 'CaveStory-rs-runs'
    output = args.output.resolve()
    if not output.is_relative_to(runs.resolve()):
        raise ValueError('Test outputs must stay in runs')
    output.mkdir(parents=True, exist_ok=True)
    sdk = Path(os.environ['LOCALAPPDATA']) / 'Android/Sdk/build-tools/35.0.1'
    java = Path('C:/Program Files/Android/Android Studio/jbr')
    env = os.environ.copy()
    env['JAVA_HOME'] = str(java)
    if not env.get('REVIA_KS_PASS'):
        raise ValueError('Provide REVIA_KS_PASS through the environment')
    test_key = output / 'untrusted-test-key.jks'
    if not test_key.exists():
        subprocess.run([str(java / 'bin/keytool.exe'), '-genkeypair', '-keystore', str(test_key), '-storepass', 'test-only-password',
                        '-keypass', 'test-only-password', '-alias', 'test', '-dname', 'CN=CaveStory Distribution Test',
                        '-keyalg', 'RSA', '-validity', '7'], check=True, capture_output=True)
    for suffix, name in [('rt', 'trusted'), ('ru', 'untrusted')]:
        package = 'io.github.cavestory_' + suffix
        unsigned, aligned, signed = (output / (name + '-' + step + '.apk') for step in ('unsigned', 'aligned', 'signed'))
        with zipfile.ZipFile(args.apk) as source, zipfile.ZipFile(unsigned, 'w') as target:
            for entry in source.infolist():
                if entry.filename.startswith('META-INF/') and (entry.filename.endswith(('.RSA', '.DSA', '.EC', '.SF')) or entry.filename == 'META-INF/MANIFEST.MF'):
                    continue
                payload = source.read(entry)
                if entry.filename == 'AndroidManifest.xml':
                    payload = clone_manifest(payload, package)
                target.writestr(entry, payload)
        subprocess.run([str(sdk / 'zipalign.exe'), '-f', '-P', '16', '4', str(unsigned), str(aligned)], check=True, capture_output=True)
        if name == 'trusted':
            key = os.environ.get('REVIA_KS_PATH', 'D:/Project/CodeX/Mod/OPCameraPro/秋风のとおり道.jks')
            passwords = ['--ks-pass', 'env:REVIA_KS_PASS', '--key-pass', 'env:REVIA_KS_PASS']
        else:
            key = str(test_key)
            passwords = ['--ks-pass', 'pass:test-only-password', '--key-pass', 'pass:test-only-password']
        subprocess.run([str(sdk / 'apksigner.bat'), 'sign', '--ks', key, *passwords, '--out', str(signed), str(aligned)], check=True, capture_output=True, env=env)
        subprocess.run([str(sdk / 'apksigner.bat'), 'verify', str(signed)], check=True, capture_output=True, env=env)
        identity = subprocess.run([str(sdk / 'aapt.exe'), 'dump', 'badging', str(signed)], check=True, capture_output=True).stdout.decode('utf-8')
        if "package: name='" + package + "'" not in identity:
            raise ValueError('Cloned APK has unexpected identity')
        print(package, signed)


if __name__ == '__main__':
    main()
