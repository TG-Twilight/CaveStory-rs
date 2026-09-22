"""Install the pinned English freeware data alongside a managed Chinese installation.

The engine extracts the retained original executable on next launch. No saves or
base files are replaced. The archive is cached under CaveStory-rs-runs/downloads.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import tempfile
import urllib.request
import zipfile
import io

ROOT = Path(__file__).resolve().parents[1]
RUNS = ROOT.parent / 'CaveStory-rs-runs'
URL = 'https://www.cavestory.one/downloads/cavestoryen.zip'
SHA256 = 'aa87fa30bee9b4980640c7e104791354e0f1f6411ee0d45a70af70046aa0685f'


def english_locale():
    result = json.loads((ROOT / 'src/data/builtin/builtin_data/locale/en.json').read_text(encoding='utf-8'))
    result.update(font='fonts/chinese-12.fnt', font_scale='1.0', encoding='shift_jis', stage_encoding='shift_jis')
    return result


def archive_bytes(archive, *, url=URL, sha256=SHA256):
    if archive.exists():
        data = archive.read_bytes()
    else:
        with urllib.request.urlopen(url, timeout=30) as response:
            if response.url != url:
                raise ValueError('Unexpected English archive redirect')
            data = response.read(2 * 1024 * 1024 + 1)
        if hashlib.sha256(data).hexdigest() != sha256:
            raise ValueError('English archive SHA-256 mismatch')
        archive.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(dir=archive.parent, delete=False) as temporary:
            temporary.write(data)
            pending = Path(temporary.name)
        pending.replace(archive)
    if hashlib.sha256(data).hexdigest() != sha256:
        raise ValueError('English archive SHA-256 mismatch')
    return data


def install(data_dir, archive):
    return install_language(data_dir, archive, code='en', language='english',
                            archive_root='CaveStory', url=URL, sha256=SHA256, locale=english_locale())


def install_language(data_dir, archive, *, code, language, archive_root, url, sha256, locale, prepare_support=None):
    data_dir = data_dir.resolve(strict=True)
    if not (data_dir / 'chinese-install.json').is_file():
        fingerprints = (ROOT / 'res/chinese/legacy-resource-hashes.properties').read_text(encoding='ascii')
        entries = [line.split('=', 1) for line in fingerprints.splitlines() if line and not line.startswith('#')]
        if (not (data_dir / 'locale/zh-Hans.json').is_file() or not (data_dir / 'fonts/chinese-12.fnt').is_file()
                or not all((data_dir / name).is_file() and hashlib.sha256((data_dir / name).read_bytes()).hexdigest() == expected
                           for name, expected in entries)):
            raise ValueError('Expected a managed Chinese installation or the verified original Chinese data')
    destination = data_dir / code
    if destination.exists():
        raise FileExistsError('Existing English resources are protected')
    contents = archive_bytes(archive, url=url, sha256=sha256)
    original = {}
    with tempfile.TemporaryDirectory(prefix=f'.{language}-install-', dir=data_dir.parent) as temporary:
        staging = Path(temporary) / code
        staging.mkdir()
        with zipfile.ZipFile(io.BytesIO(contents)) as source:
            seen = set()
            total = 0
            for entry in source.infolist():
                name = entry.filename
                path = PurePosixPath(name)
                if '\\' in name or ':' in name or path.is_absolute() or '..' in path.parts:
                    raise ValueError('Unsafe archive path')
                relative = path.relative_to(archive_root)
                if not relative.parts or entry.is_dir():
                    continue
                if relative.parts[0] == 'data':
                    relative = PurePosixPath(*relative.parts[1:])
                elif relative.parts[0] not in ('Doukutsu.exe', 'Readme.txt', 'Manual', 'Manual.html'):
                    continue
                if str(relative).lower() in seen:
                    raise ValueError('Duplicate archive member')
                seen.add(str(relative).lower())
                total += entry.file_size
                if total > 16 * 1024 * 1024:
                    raise ValueError('Archive too large')
                target = staging / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                payload = source.read(entry)
                target.write_bytes(payload)
                original[str(relative)] = hashlib.sha256(payload).hexdigest()
        for name in ('Doukutsu.exe', 'Readme.txt', 'Stage/Start.tsc', 'ArmsItem.tsc', 'Credit.tsc'):
            if not (staging / name).is_file():
                raise ValueError('Missing English resource: ' + name)
        (staging / 'locale').mkdir()
        (staging / 'locale' / f'{code}.json').write_text(json.dumps(locale, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        (staging / f'{language}-install.json').write_text(json.dumps({
            'version': 1, 'source': url, 'archive_sha256': sha256, 'original_files': original,
        }, indent=2) + '\n', encoding='utf-8')
        if prepare_support is not None:
            prepare_support(staging)
        staging.rename(destination)
    return destination


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--archive', type=Path, default=RUNS / 'downloads/cavestoryen.zip')
    args = parser.parse_args()
    print(install(args.data, args.archive))
