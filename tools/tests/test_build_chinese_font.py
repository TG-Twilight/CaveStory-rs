import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

SCRIPT = Path(__file__).resolve().parents[1] / 'build_chinese_font.py'
SPEC = importlib.util.spec_from_file_location('build_chinese_font', SCRIPT)
font = importlib.util.module_from_spec(SPEC) if SPEC else None
if SCRIPT.exists():
    SPEC.loader.exec_module(font)

# Asymmetric glyph with a descender: catches mirrored rows and wrong baseline.
BDF = '''STARTFONT 2.1
SIZE 12 75 75
FONT_ASCENT 10
FONT_DESCENT 2
CHARS 2
STARTCHAR space
ENCODING 32
DWIDTH 6 0
BBX 0 0 0 0
BITMAP
ENDCHAR
STARTCHAR test
ENCODING 20013
DWIDTH 12 0
BBX 3 2 1 -1
BITMAP
80
60
ENDCHAR
ENDFONT
'''


class FontTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(hasattr(font, 'parse_bdf'), 'BDF converter is not implemented')

    def test_preserves_pixels_advance_and_descender(self):
        ascent, descent, glyphs = font.parse_bdf(BDF)
        self.assertEqual((ascent, descent), (10, 2))
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'font'
            font.write_bmfont(output, ascent, descent, glyphs)
            data = (output / 'chinese-12.fnt').read_bytes()
            self.assertEqual(data[:4], b'BMF\x03')
            blocks = {}
            pos = 4
            while pos < len(data):
                block, size = struct.unpack_from('<BI', data, pos)
                blocks[block] = data[pos + 5:pos + 5 + size]
                pos += 5 + size
            records = list(struct.iter_unpack('<IHHHHhhhBB', blocks[4]))
            self.assertEqual(records[0][0], 32)
            self.assertEqual(records[0][7], 6)
            char, x, y, w, h, dx, dy, advance, page, channel = records[1]
            self.assertEqual((char, w, h, dx, dy, advance, channel), (20013, 3, 2, 1, 9, 12, 15))
            with Image.open(output / f'chinese-12_{page}.png') as atlas:
                self.assertEqual(list(atlas.crop((x, y, x + w, y + h)).getchannel('A').tobytes()),
                                 [255, 0, 0, 0, 255, 255])

    def test_refuses_existing_output_and_preserves_sentinel(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)
            sentinel = output / 'save.dat'
            sentinel.write_bytes(b'existing save')
            with self.assertRaises(FileExistsError):
                font.write_bmfont(output, *font.parse_bdf(BDF))
            self.assertEqual(sentinel.read_bytes(), b'existing save')

    def test_rejects_truncated_bitmap(self):
        with self.assertRaises(ValueError):
            font.parse_bdf(BDF.replace('80\n60', '80'))

    def test_accepts_font_icons_wider_than_nominal_size(self):
        wide = BDF.replace('BBX 3 2 1 -1', 'BBX 27 2 1 -1').replace('80\n60', '80000000\n00000020')
        metrics = font.parse_bdf(wide)
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'font'
            font.write_bmfont(output, *metrics)
            with Image.open(output / 'chinese-12_0.png') as atlas:
                left, top, right, bottom = atlas.getchannel('A').getbbox()
                self.assertEqual((right - left, bottom - top), (27, 2))

    def test_rejects_duplicate_codepoints(self):
        with self.assertRaises(ValueError):
            font.parse_bdf(BDF.replace('ENCODING 32', 'ENCODING 20013'))

    def test_rejects_wrong_archive_before_writing(self):
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / 'font.zip'
            archive.write_bytes(b'not the pinned font')
            output = Path(tmp) / 'output'
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                font.build(archive, output)
            self.assertFalse(output.exists())

    def test_wraps_rows_and_pages_without_losing_pixels(self):
        glyph = '''STARTCHAR test
ENCODING {code}
DWIDTH 8 0
BBX 7 7 0 0
BITMAP
FE
FE
FE
FE
FE
FE
FE
ENDCHAR
'''
        bdf = 'STARTFONT 2.1\nFONT_ASCENT 10\nFONT_DESCENT 2\nCHARS 5\n'
        bdf += ''.join(glyph.format(code=i) for i in range(65, 70)) + 'ENDFONT\n'
        with tempfile.TemporaryDirectory() as tmp, patch.object(font, 'PAGE_SIZE', 20):
            output = Path(tmp) / 'font'
            font.write_bmfont(output, *font.parse_bdf(bdf))
            data = (output / 'chinese-12.fnt').read_bytes()
            pos = 4
            while data[pos] != 4:
                pos += 5 + struct.unpack_from('<I', data, pos + 1)[0]
            records = list(struct.iter_unpack('<IHHHHhhhBB', data[pos + 5:]))
            self.assertEqual([(r[1], r[2], r[8]) for r in records],
                             [(1, 1, 0), (10, 1, 0), (1, 10, 0), (10, 10, 0), (1, 1, 1)])
            for r in records:
                with Image.open(output / f'chinese-12_{r[8]}.png') as atlas:
                    self.assertEqual(atlas.crop((r[1], r[2], r[1] + 7, r[2] + 7)).getchannel('A').tobytes(),
                                     bytes([255]) * 49)


if __name__ == '__main__':
    unittest.main()
