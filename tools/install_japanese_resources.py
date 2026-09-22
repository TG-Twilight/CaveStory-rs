"""Install the verified Japanese freeware beside existing managed game resources."""
import argparse
import json
from pathlib import Path

import install_english_resources as shared

ROOT = shared.ROOT
RUNS = shared.RUNS
URL = 'https://studiopixel.jp/binaries/dou_1006.zip'
SHA256 = '68d0dfab0afa2bfb0c6f750638d6a8883576cbbe33f65be8ff51d74272d0eea0'


def japanese_locale():
    result = json.loads((ROOT / 'src/data/builtin/builtin_data/locale/jp.json').read_text(encoding='utf-8'))
    result.update(font='fonts/japanese/chinese-12.fnt', font_scale='1.0',
                  encoding='shift_jis', stage_encoding='shift_jis')
    return result


def archive_bytes(archive):
    return shared.archive_bytes(archive, url=URL, sha256=SHA256)


def install(data_dir, archive):
    def prepare_font(staging):
        import build_chinese_font as font
        font_archive = font.ensure_archive(RUNS / 'downloads' / font.URL.rsplit('/', 1)[1])
        font.build(font_archive, staging / 'fonts/japanese', japanese=True)

    return shared.install_language(data_dir, archive, code='jp', language='japanese',
                                   archive_root='doukutsu', url=URL, sha256=SHA256, locale=japanese_locale(),
                                   prepare_support=prepare_font)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--archive', type=Path, default=RUNS / 'downloads/dou_1006.zip')
    args = parser.parse_args()
    print(install(args.data, args.archive))
