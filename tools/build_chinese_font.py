"""Build an external BMFont overlay from the pinned OFL Fusion Pixel BDF archive.

No game data is read or distributed. The archive is supplied by the caller;
only explicitly named members are read, and an existing output is never reused.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import urllib.request
import zipfile

from PIL import Image, __version__ as pillow_version

VERSION = '2026.09.01'
URL = ('https://github.com/TakWolf/fusion-pixel-font/releases/download/'
       f'{VERSION}/fusion-pixel-font-12px-monospaced-bdf-v{VERSION}.zip')
SHA256 = '5b1cac9253fa9e20b9fea5fd582eeed985819a3ce4b7f7fb108f8d6e599ad211'
BDF_NAME = 'fusion-pixel-12px-monospaced-zh_hans.bdf'
LICENSES = ('OFL.txt', 'LICENSES/ark-pixel/OFL.txt',
            'LICENSES/cubic-11/OFL.txt', 'LICENSES/galmuri/LICENSE.txt')
FONT_NAME = 'chinese-12'
PAGE_SIZE = 1024


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def ensure_archive(archive):
    """Fetch a pinned font once; never publish a partial or unverified cache."""
    if not archive.exists():
        archive.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(URL, timeout=30) as response:
            data = response.read(32 * 1024 * 1024 + 1)
        if sha256(data) != SHA256:
            raise ValueError('Font download SHA-256 mismatch')
        with tempfile.NamedTemporaryFile(dir=archive.parent, delete=False) as pending:
            pending.write(data)
            path = Path(pending.name)
        path.replace(archive)
    if sha256(archive.read_bytes()) != SHA256:
        raise ValueError('Font archive SHA-256 mismatch')
    return archive


def parse_bdf(text):
    """Read the bitmap/metrics subset used by the pinned 12px BDF, strictly."""
    ascent = descent = count = None
    glyphs = {}
    glyph = None
    bitmap = False
    seen = 0
    ended = False
    for line in text.splitlines():
        fields = line.split()
        if not fields:
            continue
        key = fields[0]
        if key == 'FONT_ASCENT':
            ascent = int(fields[1])
        elif key == 'FONT_DESCENT':
            descent = int(fields[1])
        elif key == 'CHARS':
            count = int(fields[1])
        elif key == 'STARTCHAR':
            if glyph is not None:
                raise ValueError('Unterminated BDF glyph')
            glyph = {'rows': []}
        elif key == 'ENDCHAR':
            if glyph is None or not bitmap or not {'code', 'advance', 'box'} <= glyph.keys():
                raise ValueError('Incomplete BDF glyph')
            width, height, _, _ = glyph['box']
            # The 12px font includes a few taller symbols and wider private-use icons.
            if not (0 <= width <= PAGE_SIZE - 2 and 0 <= height <= PAGE_SIZE - 2):
                raise ValueError('BDF glyph does not fit an atlas page')
            if len(glyph['rows']) != height:
                raise ValueError('Incorrect BDF bitmap row count')
            if any(len(row) != (width + 7) // 8 for row in glyph['rows']):
                raise ValueError('Incorrect BDF bitmap row width')
            code = glyph['code']
            if code >= 0:
                if code > 0x10ffff or 0xd800 <= code <= 0xdfff or code in glyphs:
                    raise ValueError('Invalid or duplicate BDF codepoint')
                glyphs[code] = glyph
            seen += 1
            glyph = None
            bitmap = False
        elif key == 'ENDFONT':
            ended = True
        elif glyph is not None:
            if key == 'ENCODING':
                glyph['code'] = int(fields[1])
            elif key == 'DWIDTH':
                glyph['advance'] = int(fields[1])
                if int(fields[2]) != 0:
                    raise ValueError('Vertical BDF glyphs are unsupported')
            elif key == 'BBX':
                glyph['box'] = tuple(map(int, fields[1:]))
                if len(glyph['box']) != 4:
                    raise ValueError('Invalid BDF bounding box')
            elif key == 'BITMAP':
                bitmap = True
            elif bitmap:
                glyph['rows'].append(bytes.fromhex(line))
    if not ended or glyph is not None or count != seen or not glyphs:
        raise ValueError('Incomplete BDF font')
    if ascent is None or descent is None or ascent < 0 or descent < 0 or ascent + descent != 12:
        raise ValueError('Expected a 12px BDF line height')
    return ascent, descent, glyphs


def write_bmfont(output, ascent, descent, glyphs):
    """Write BMF v3 with the stem_N.png naming required by the engine."""
    output.mkdir(parents=True, exist_ok=False)
    pages = []
    records = bytearray()
    x = y = 1
    row_height = 0
    for code, glyph in sorted(glyphs.items()):
        width, height, dx, bottom = glyph['box']
        if x + width + 1 > PAGE_SIZE:
            x = 1
            y += row_height + 2
            row_height = 0
        if not pages or y + height + 1 > PAGE_SIZE:
            if len(pages) >= 256:
                raise ValueError('BMF v3 supports at most 256 atlas pages')
            pages.append(Image.new('RGBA', (PAGE_SIZE, PAGE_SIZE), (255, 255, 255, 0)))
            x = y = 1
            row_height = 0
        page = len(pages) - 1
        for row_index, row in enumerate(glyph['rows']):
            for column in range(width):
                if row[column // 8] & (0x80 >> (column % 8)):
                    pages[page].putpixel((x + column, y + row_index), (255, 255, 255, 255))
        records.extend(struct.pack('<IHHHHhhhBB', code, x, y, width, height,
                                   dx, ascent - height - bottom, glyph['advance'], page, 15))
        x += width + 2
        row_height = max(row_height, height)
    names = [f'{FONT_NAME}_{i}.png' for i in range(len(pages))]
    info = struct.pack('<hBBHBBBBBBBB', 12, 2, 0, 100, 1, 0, 0, 0, 0, 0, 0, 0)
    info += b'CaveStory Chinese 12\0'  # Distinct name for the converted font.
    common = struct.pack('<HHHHHBBBBB', ascent + descent, ascent, PAGE_SIZE, PAGE_SIZE,
                         len(pages), 0, 0, 0, 0, 0)
    result = bytearray(b'BMF\x03')
    for kind, block in ((1, info), (2, common),
                        (3, b''.join(name.encode('ascii') + b'\0' for name in names)), (4, records)):
        result.extend(struct.pack('<BI', kind, len(block)))
        result.extend(block)
    (output / f'{FONT_NAME}.fnt').write_bytes(result)
    for name, atlas in zip(names, pages):
        atlas.save(output / name, compress_level=9)


def build(archive, output, *, japanese=False):
    archive_data = archive.read_bytes()
    if sha256(archive_data) != SHA256:
        raise ValueError('Font archive SHA-256 mismatch; expected ' + SHA256)
    bdf_name = 'fusion-pixel-12px-monospaced-ja.bdf' if japanese else BDF_NAME
    with zipfile.ZipFile(archive) as source:
        # No extractall: archive paths cannot become arbitrary output paths.
        inputs = {name: source.read(name) for name in (bdf_name, *LICENSES)}
    ascent, descent, glyphs = parse_bdf(inputs[bdf_name].decode('utf-8'))
    write_bmfont(output, ascent, descent, glyphs)
    for name in LICENSES:
        path = output / 'licenses' / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(inputs[name])
    manifest = {
        'source_url': URL, 'source_version': VERSION, 'source_archive_sha256': SHA256,
        'source_members_sha256': {name: sha256(data) for name, data in inputs.items()},
        'generator_sha256': sha256(Path(__file__).read_bytes()), 'pillow_version': pillow_version,
        'format': 'BMF v3', 'font_name': 'CaveStory Chinese 12', 'locale': 'jp' if japanese else 'zh-Hans',
        'glyphs': len(glyphs), 'line_height': ascent + descent, 'font_scale': 1.0,
        'page_size': PAGE_SIZE, 'packing': 'sorted codepoints, shelf rows, 1px padding',
        'outputs_sha256': {p.relative_to(output).as_posix(): sha256(p.read_bytes())
                           for p in sorted(output.rglob('*')) if p.is_file()},
    }
    (output / 'font-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path, help='New output directory; must not exist')
    parser.add_argument('--japanese', action='store_true', help='Use Japanese regional glyphs from the same pinned font')
    args = parser.parse_args()
    try:
        manifest = build(args.archive, args.output, japanese=args.japanese)
    except (ValueError, OSError, KeyError, zipfile.BadZipFile) as error:
        parser.exit(1, f'Font build failed: {error}\n')
    print(f"Built {manifest['glyphs']} glyphs in {args.output}")


if __name__ == '__main__':
    main()
