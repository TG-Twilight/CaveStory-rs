"""Patch only the pinned freeware Barr.tsc; preserve originals and custom scripts.

The repository stores our added dialogue and fingerprints, never the original
game script. Android consumes the same encoded dialogue fragments.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / 'res/distribution'
LANGUAGES = {
    'zh-Hans': ('gbk', '92d511abd74d76e8e2da26cd1b717ad605dbb0019c13e858b81042247b52366a'),
    'en': ('cp932', '7b8e3584beb5fe7f12089b75baec8ab8f342cc34d52d4396704d170cbf3ad96c'),
    'jp': ('cp932', '5660c0518a274ee5d8c25d0f1a52bfd220a8d7a040840adc3ebf3693f876977a'),
}
ANCHOR = b'<FAC0005<MSG'
HOOK = b'<PSH9500'
PREVIOUS = {
    'zh-Hans': 'f1ae776b375953b55692aee6690515e8757dc0f5d0878c04a0814653cbec6467',
    'en': '22421e029777b66b603d3a9faaf01fbd40f0443a5bca6c09344699c3cdccd974',
    'jp': '070d4dd1c53016bd4057f984483f16787bed8b15bd003169da3e733761ea6f29',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def decrypt(raw):
    mid = len(raw) // 2
    key = raw[mid] or 7
    return bytes(b if i == mid else (b - key) & 255 for i, b in enumerate(raw))


def encrypt(plain):
    mid = len(plain) // 2
    key = plain[mid] or 7
    return bytes(b if i == mid else (b + key) & 255 for i, b in enumerate(plain))


def fragment(language):
    return (SCRIPTS / (language + '.txt')).read_text(encoding='utf-8').replace('\r\n', '\n').encode(LANGUAGES[language][0])


def original_bytes(raw, language):
    if digest(raw) == LANGUAGES[language][1]:
        return raw
    if digest(raw) != PREVIOUS[language]:
        return None
    # Exact shipped revisions only; edits to the story or notice are preserved.
    plain = decrypt(raw).split(b'\n#9500', 1)[0]
    original = encrypt(plain.replace(HOOK, b'', 1))
    if digest(original) != LANGUAGES[language][1]:
        raise ValueError('Previous notice does not contain the pinned original')
    return original


def patch_bytes(raw, language):
    original = original_bytes(raw, language)
    if original is None:
        return raw
    plain = decrypt(original)
    if plain.count(ANCHOR) != 1 or b'#9500' in plain:
        raise ValueError('Unexpected Balrog event layout')
    start = plain.index(ANCHOR)
    challenge = plain.index(b'<YNJ1001', start)
    # After his complaint and pause, before his original address to the player.
    offset = plain.rindex(b'<CLR', start, challenge) + len(b'<CLR')
    return encrypt(plain[:offset] + HOOK + plain[offset:] + b'\n' + fragment(language))


def install_language(data, language):
    data = Path(data)
    target = data / 'Stage/Barr.tsc'
    if not target.is_file() or target.stat().st_size > 128 * 1024 or target.is_symlink() or target.parent.is_symlink():
        return None
    raw = target.read_bytes()
    patched = patch_bytes(raw, language)
    if raw == patched:
        return None
    original = original_bytes(raw, language)
    backup_dir = data / '.distribution-notice'
    if backup_dir.is_symlink():
        raise ValueError('Refusing a linked backup directory')
    backup_dir.mkdir(exist_ok=True)
    backup = backup_dir / 'Barr.tsc.original'
    if backup.exists():
        if backup.is_symlink() or backup.read_bytes() != original:
            raise ValueError('Existing original backup differs')
    else:
        with tempfile.NamedTemporaryFile(dir=backup_dir, prefix='.original-', delete=False) as stream:
            temporary_backup = Path(stream.name)
            try:
                stream.write(original)
                stream.flush()
                os.fsync(stream.fileno())
            except BaseException:
                stream.close()
                temporary_backup.unlink(missing_ok=True)
                raise
        try:
            if temporary_backup.read_bytes() != original:
                raise ValueError('Original backup verification failed')
            # Atomic publication without replacing an existing backup.
            os.link(temporary_backup, backup)
        finally:
            temporary_backup.unlink(missing_ok=True)
    record = {'version': 2, 'path': 'Stage/Barr.tsc', 'language': language,
              'original_sha256': digest(original),
              'before_sha256': digest(raw), 'after_sha256': digest(patched)}
    with tempfile.NamedTemporaryFile(dir=target.parent, prefix='.distribution-', delete=False) as pending:
        temporary = Path(pending.name)
        pending.write(patched)
        pending.flush()
        os.fsync(pending.fileno())
    try:
        if target.read_bytes() != raw or temporary.read_bytes() != patched:
            raise ValueError('Script changed while preparing patch')
        temporary.replace(target)
    finally:
        temporary.unlink(missing_ok=True)
    (backup_dir / 'manifest.json').write_text(json.dumps(record, indent=2) + '\n', encoding='utf-8')
    return record


def prepare_assets(output):
    output = Path(output) / 'distribution'
    output.mkdir(parents=True, exist_ok=True)
    for language, (_, expected) in LANGUAGES.items():
        (output / (language + '.bin')).write_bytes(fragment(language))
    (output / 'originals.properties').write_text(''.join(
        f'{code}={v[1]}\n{code}.previous={PREVIOUS[code]}\n' for code, v in LANGUAGES.items()), encoding='ascii')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    args = parser.parse_args()
    for language, relative in [('zh-Hans', ''), ('en', 'en'), ('jp', 'jp')]:
        print(language, install_language(args.data / relative, language))
