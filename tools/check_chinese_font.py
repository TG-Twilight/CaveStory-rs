"""Read-only coverage audit for the verified Windows simplified Chinese data.

Checks menu keys/placeholders and the union of scripts, map names, and menus.
Does not prove that the engine loads the data or that text fits on screen.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import struct

ROOT = Path(__file__).resolve().parents[1]
EXE_SHA256 = 'a91d270b64984ad52f374688e2b7fb82c83f138d5e93fd8f35b325f3229e8806'
LOCALE = ROOT / 'res/chinese/locale/zh-Hans.json'
ENGLISH = ROOT / 'src/data/builtin/builtin_data/locale/en.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def flatten(value, prefix=''):
    result = {}
    for key, item in value.items():
        if isinstance(item, dict):
            result.update(flatten(item, prefix + key + '.'))
        elif isinstance(item, str):
            result[prefix + key] = item
    return result


def font_characters(data):
    if data[:4] != b'BMF\x03':
        raise ValueError('Expected BMF v3 font')
    pos = 4
    glyphs = set()
    while pos < len(data):
        kind, size = struct.unpack_from('<BI', data, pos)
        pos += 5
        if pos + size > len(data):
            raise ValueError('Truncated BMF block')
        if kind == 4:
            if size % 20:
                raise ValueError('Invalid BMF character block size')
            # Match BMFontMetadata::load_from: invalid Unicode ids are skipped.
            glyphs.update(chr(item[0]) for item in struct.iter_unpack('<IHHHHhhhBB', data[pos:pos + size])
                          if item[0] <= 0x10ffff and not 0xd800 <= item[0] <= 0xdfff)
        pos += size
    return glyphs


def validate_locale(chinese, english):
    labels = {'game.inventory.weapons', 'game.inventory.items', 'game.hud.air',
              'game.caret.level_up', 'game.caret.level_down', 'game.caret.empty',
              'game.hud.level', 'game.hud.max', 'game.stage_select', 'game.boss'}
    extensions = {'encoding', 'stage_encoding'} | labels
    if chinese.keys() - extensions != english.keys() - extensions:
        raise ValueError('Chinese locale keys do not match English locale')
    for key in english:
        if Counter(re.findall(r'\{[^{}]+\}', chinese[key])) != Counter(re.findall(r'\{[^{}]+\}', english[key])):
            raise ValueError('Locale placeholder mismatch: ' + key)
    for key in labels & chinese.keys():
        if not chinese[key].strip() or re.search(r'\{[^{}]+\}', chinese[key]):
            raise ValueError('Image label must be nonempty plain text: ' + key)


def audit(source, font_path):
    chinese = flatten(json.loads(LOCALE.read_text(encoding='utf-8')))
    english = flatten(json.loads(ENGLISH.read_text(encoding='utf-8')))
    validate_locale(chinese, english)
    exe = (source / 'Doukutsu.exe').read_bytes()
    if digest(exe) != EXE_SHA256:
        raise ValueError('Unrecognized simplified EXE; fixed map-table offset is unsafe')
    # Confirmed against VanillaExtractor/PE addressing for this exact EXE only.
    maps = [exe[0x937b0 + i * 200 + 165:0x937b0 + i * 200 + 197]
            .split(b'\0')[0].decode('gbk', errors='strict') for i in range(95)]
    script_chars = set()
    scripts = {}
    for path in sorted((source / 'data').rglob('*.tsc')):
        raw = path.read_bytes()
        if not raw:
            raise ValueError('Empty TSC: ' + str(path))
        middle = len(raw) // 2
        key = raw[middle] or 7
        decrypted = bytes(b if i == middle else (b - key) & 255 for i, b in enumerate(raw))
        script_chars.update(decrypted.decode('gbk', errors='strict'))
        scripts[path.relative_to(source).as_posix()] = digest(raw)
    if len(scripts) != 99:
        raise ValueError(f'Expected 99 simplified scripts, found {len(scripts)}')
    categories = {
        'scripts': script_chars,
        'maps': set(''.join(maps)),
        'menus': set(''.join(v for k, v in chinese.items()
                             if k not in {'font', 'font_scale', 'encoding', 'stage_encoding'})),
    }
    data = font_path.read_bytes()
    glyphs = font_characters(data)
    required = set.union(*categories.values()) - set('\r\n\t')
    missing = required - glyphs
    return {
        'scope': 'Static GBK decoding and font coverage; runtime and proofreading remain unverified',
        'source': str(source.resolve()), 'exe_sha256': digest(exe),
        'scripts_sha256': scripts, 'script_count': len(scripts), 'map_count': len(maps),
        'locale_sha256': digest(LOCALE.read_bytes()), 'locale_keys': len(chinese),
        'font_sha256': digest(data), 'font_glyphs': len(glyphs),
        'required_characters': len(required),
        'missing': [f'U+{ord(c):04X} {c}' for c in sorted(missing)],
        'categories': {name: {'characters': len(chars - set('\r\n\t')),
                              'missing': len((chars - set('\r\n\t')) - glyphs)}
                       for name, chars in categories.items()},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path, help='Directory containing Doukutsu.exe and data/')
    parser.add_argument('--font', required=True, type=Path)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    try:
        report = audit(args.source, args.font)
        text = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
        if args.report:
            # Reports are new files so a typo cannot overwrite a source or save.
            with args.report.open('x', encoding='utf-8') as output:
                output.write(text)
        print(json.dumps({k: v for k, v in report.items() if k != 'scripts_sha256'}, ensure_ascii=True, indent=2))
    except (ValueError, OSError, struct.error) as error:
        parser.exit(1, f'Audit failed: {error}\n')
    if report['missing']:
        parser.exit(1, 'Font is missing required characters\n')


if __name__ == '__main__':
    main()
