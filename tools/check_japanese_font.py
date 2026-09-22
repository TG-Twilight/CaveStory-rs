"""Audit pinned Japanese story/map decoding and Japanese BMFont coverage."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zipfile

import check_chinese_font as common
import install_japanese_resources as japanese


def audit(archive, font):
    payload = japanese.archive_bytes(archive)
    with zipfile.ZipFile(archive) as source:
        exe = source.read('doukutsu/Doukutsu.exe')
        pe = struct.unpack_from('<I', exe, 0x3c)[0]
        sections, optional_size = struct.unpack_from('<H12xH', exe, pe + 6)
        optional = pe + 24
        image_base = struct.unpack_from('<I', exe, optional + 28)[0]
        pattern = bytes.fromhex('83c4088b450869c0c800000005')
        offset = exe.index(pattern)
        address = struct.unpack_from('<I', exe, offset + len(pattern))[0] - image_base
        table_offset = None
        for i in range(sections):
            header = optional + optional_size + i * 40
            size, virtual, raw_size, raw = struct.unpack_from('<IIII', exe, header + 8)
            if virtual <= address and address + 19000 <= virtual + raw_size:
                table_offset = raw + address - virtual
                break
        if table_offset is None:
            raise ValueError('Japanese map table not found')
        maps = [exe[table_offset + i * 200 + 165:table_offset + i * 200 + 197]
                .split(b'\0')[0].decode('shift_jis', errors='strict') for i in range(95)]
        scripts = {}
        chars = set()
        for name in source.namelist():
            if name.startswith('doukutsu/data/') and name.lower().endswith('.tsc'):
                raw = source.read(name)
                middle = len(raw) // 2
                key = raw[middle] or 7
                decrypted = bytes(b if i == middle else (b - key) & 255 for i, b in enumerate(raw))
                # encoding_rs::SHIFT_JIS used by the engine includes Windows
                # extensions: Statue.tsc contains 0x8754 (Roman numeral I).
                chars.update(decrypted.decode('cp932', errors='strict'))
                scripts[name] = hashlib.sha256(raw).hexdigest()
        if len(scripts) != 99:
            raise ValueError('Expected 99 Japanese TSC files')
    locale = common.flatten(japanese.japanese_locale())
    english = common.flatten(json.loads(common.ENGLISH.read_text(encoding='utf-8')))
    common.validate_locale(locale, english)
    glyphs = common.font_characters(font.read_bytes())
    chars.update(''.join(maps))
    chars.update(''.join(locale.values()))
    # The language picker must show the other languages in their own names.
    chars.update('简体中文English日本語')
    required = chars - set('\r\n\t')
    return {'archive_sha256': hashlib.sha256(payload).hexdigest(),
            'exe_sha256': hashlib.sha256(exe).hexdigest(), 'table_offset': table_offset,
            'encoding': 'shift_jis (Windows extensions / CP932)', 'script_count': len(scripts), 'map_count': len(maps),
            'map_names': maps, 'scripts_sha256': scripts, 'locale_keys': len(locale),
            'font_sha256': hashlib.sha256(font.read_bytes()).hexdigest(),
            'font_glyphs': len(glyphs), 'required_characters': len(required),
            'missing': [f'U+{ord(c):04X} {c}' for c in sorted(required - glyphs)]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, default=japanese.RUNS / 'downloads/dou_1006.zip')
    parser.add_argument('--font', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.archive, args.font)
    args.report.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k not in ('map_names', 'scripts_sha256')}, ensure_ascii=True))
    if result['missing']:
        raise SystemExit('Missing Japanese glyphs')
