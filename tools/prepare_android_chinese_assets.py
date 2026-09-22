"""Build APK support assets only; never include or download Cave Story game data."""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile
import urllib.request
import zipfile

import build_chinese_font as font
import install_japanese_resources as japanese
import patch_distribution_notice as notice

ROOT = Path(__file__).resolve().parents[1]


def prepare(archive, output):
    if not archive.exists():
        archive.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(font.URL, timeout=30) as response:
            data = response.read(32 * 1024 * 1024 + 1)
        if hashlib.sha256(data).hexdigest() != font.SHA256:
            raise ValueError('Font download SHA-256 mismatch')
        # An interrupted download must not leave a cache that looks complete.
        with tempfile.NamedTemporaryFile(dir=archive.parent, delete=False) as temporary:
            temporary.write(data)
            temporary_path = Path(temporary.name)
        temporary_path.replace(archive)
    output.mkdir(parents=True, exist_ok=True)
    notice.prepare_assets(output)
    (output / 'legacy-resource-hashes.properties').write_bytes((ROOT / 'res/chinese/legacy-resource-hashes.properties').read_bytes())
    # Standalone metadata for the optional downloaded English story pack.
    english = json.loads((ROOT / 'src/data/builtin/builtin_data/locale/en.json').read_text(encoding='utf-8'))
    english.update(font='fonts/chinese-12.fnt', font_scale='1.0', encoding='shift_jis', stage_encoding='shift_jis')
    (output / 'english-locale.json').write_text(json.dumps(english, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    (output / 'japanese-locale.json').write_text(json.dumps(japanese.japanese_locale(), ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    with tempfile.TemporaryDirectory(prefix='cavestory-font-', dir=output) as temporary:
        staging = Path(temporary)
        font.build(archive, staging / 'fonts')
        font.build(archive, staging / 'fonts/japanese', japanese=True)
        # A separate overlay lets an existing installation acquire Japanese fonts
        # inside its new jp directory without changing shared or custom fonts.
        with zipfile.ZipFile(output / 'japanese-fonts.zip', 'w', zipfile.ZIP_DEFLATED) as target:
            for path in sorted((staging / 'fonts/japanese').rglob('*')):
                if path.is_file():
                    info = zipfile.ZipInfo(path.relative_to(staging).as_posix(), (2026, 9, 1, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_DEFLATED
                    target.writestr(info, path.read_bytes())
        (staging / 'locale').mkdir()
        for code in ('zh-Hans', 'en', 'jp'):
            source = (ROOT / 'res/chinese/locale/zh-Hans.json' if code == 'zh-Hans' else
                      ROOT / f'src/data/builtin/builtin_data/locale/{code}.json')
            locale = json.loads(source.read_text(encoding='utf-8'))
            # These downloads contain Chinese story data even with English/Japanese menus.
            locale.update(font='fonts/chinese-12.fnt', font_scale='1.0', encoding='gbk', stage_encoding='gbk')
            (staging / 'locale' / f'{code}.json').write_text(
                json.dumps(locale, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        (staging / 'zh-Hans').mkdir()
        (staging / 'zh-Hans' / 'installed.txt').write_text('Chinese story is in the base data directory.\n')
        (staging / 'chinese-install.json').write_text(json.dumps({
            'version': 1, 'default_locale': 'zh-Hans', 'story_encoding': 'gbk',
            'source': 'https://www.cavestory.one/downloads/dou_sc102.zip',
            'archive_sha256': 'd1e632a8f88cbd704ad300a27fd74abe2ada8faa67789d2274dd8ef8fee4df1e',
        }, indent=2) + '\n', encoding='utf-8')
        # Fixed ZIP timestamps and sorted paths make the archive repeatable.
        with zipfile.ZipFile(output / 'chinese-overlay.zip', 'w', zipfile.ZIP_DEFLATED) as target:
            for path in sorted(staging.rglob('*')):
                if path.is_file():
                    info = zipfile.ZipInfo(path.relative_to(staging).as_posix(), (2026, 9, 1, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_DEFLATED
                    target.writestr(info, path.read_bytes())


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, default=ROOT.parent / 'CaveStory-rs-runs/downloads' / font.URL.rsplit('/', 1)[1])
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    prepare(args.archive, args.output)
