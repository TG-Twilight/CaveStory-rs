"""Build a local-only three-language package; keep original game data out of engine releases."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import struct
import zipfile
import urllib.request
from datetime import datetime
import install_japanese_resources as japanese

import install_english_resources as english
import prepare_android_chinese_assets as support
import patch_distribution_notice as notice

ROOT = Path(__file__).resolve().parents[1]
HANS_HASH = 'd1e632a8f88cbd704ad300a27fd74abe2ada8faa67789d2274dd8ef8fee4df1e'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


MACHINES = {'x86_64': 0x8664, 'x86_32': 0x14c, 'arm64': 0xaa64}


def pe_machine(payload):
    if payload[:2] != b'MZ':
        raise ValueError('Missing DOS header')
    pe = struct.unpack_from('<I', payload, 0x3c)[0]
    if payload[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Missing PE header')
    return struct.unpack_from('<H', payload, pe + 4)[0]


def prepare_archives(output):
    game = output / 'game'
    game.mkdir(parents=True, exist_ok=True)
    chinese = english.RUNS / 'downloads/dou_sc102.zip'
    if not chinese.exists():
        with urllib.request.urlopen('https://www.cavestory.one/downloads/dou_sc102.zip', timeout=30) as response:
            payload = response.read(4 * 1024 * 1024)
        if hashlib.sha256(payload).hexdigest() != HANS_HASH:
            raise ValueError('Chinese source SHA-256 mismatch')
        chinese.parent.mkdir(parents=True, exist_ok=True)
        chinese.write_bytes(payload)
    if sha(chinese) != HANS_HASH:
        raise ValueError('Chinese source SHA-256 mismatch')
    english.archive_bytes(english.RUNS / 'downloads/cavestoryen.zip')
    japanese.archive_bytes(english.RUNS / 'downloads/dou_1006.zip')
    sources = {}
    for name, url, expected in [('dou_sc102.zip', 'https://www.cavestory.one/downloads/dou_sc102.zip', HANS_HASH),
                                ('cavestoryen.zip', english.URL, english.SHA256), ('dou_1006.zip', japanese.URL, japanese.SHA256)]:
        source = english.RUNS / 'downloads' / name
        if sha(source) != expected:
            raise ValueError('Archive mismatch: ' + name)
        shutil.copy2(source, game / name)
        sources[name] = {'source': url, 'sha256': expected, 'bytes': source.stat().st_size}
    (game / 'source-manifest.json').write_text(json.dumps(sources, indent=2) + '\n', encoding='utf-8')


def runtime_files(arch):
    redist_arch = {'x86_64': 'x64', 'x86_32': 'x86', 'arm64': 'arm64'}[arch]
    candidates = [english.RUNS / 'cache/msvc-runtime' / arch,
                  Path('C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Redist/MSVC/14.29.30133') / redist_arch / 'Microsoft.VC142.CRT']
    for directory in candidates:
        if (directory / 'vcruntime140.dll').exists():
            files = sorted(directory.glob('*.dll'))
            for path in files:
                if pe_machine(path.read_bytes()) != MACHINES[arch]:
                    raise ValueError('Wrong runtime architecture: ' + str(path))
            return files
    raise FileNotFoundError('Missing matching CRT runtime: ' + arch)


def package(engine, output, arch, date):
    datetime.strptime(date, '%Y%m%d')
    output.mkdir(parents=True, exist_ok=False)
    staging = output / ('staging/CaveStory-rs_windows_' + date + '_' + arch + '_game')
    staging.mkdir(parents=True)
    archive = english.RUNS / 'downloads/dou_sc102.zip'
    if sha(archive) != HANS_HASH:
        raise ValueError('Chinese source SHA-256 mismatch')
    originals = {}
    with zipfile.ZipFile(archive) as source:
        for entry in source.infolist():
            name = entry.filename
            relative = PurePosixPath(name).relative_to('Doukutsu')
            if '..' in relative.parts or '\\' in name or ':' in name:
                raise ValueError('Unsafe source path')
            if entry.is_dir() or not relative.parts or relative.parts[0] not in ('data', 'Manual', 'Manual.html', 'Readme.txt', 'Doukutsu.exe'):
                continue
            target = staging / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.read(entry))
            originals[str(relative)] = sha(target)
    payload = engine.read_bytes()
    pe = struct.unpack_from('<I', payload, 0x3c)[0]
    if pe_machine(payload) != MACHINES[arch]:
        raise ValueError('Wrong engine architecture')
    shutil.copy2(engine, staging / 'CaveStory-rs.exe')
    shutil.copy2(ROOT / 'LICENSE', staging / 'LICENSE-doukutsu-rs.txt')
    shutil.copy2(ROOT / 'AUTHORS.md', staging / 'AUTHORS-doukutsu-rs.md')
    shutil.copy2(ROOT / 'vendor/trainer/notices/TRAINER-LICENSES.txt', staging / 'TRAINER-LICENSES.txt')
    support_dir = output / 'support'
    support.prepare(english.RUNS / 'downloads' / support.font.URL.rsplit('/', 1)[1], support_dir)
    with zipfile.ZipFile(support_dir / 'chinese-overlay.zip') as overlay:
        for entry in overlay.infolist():
            target = staging / 'data' / entry.filename
            if not target.resolve().is_relative_to((staging / 'data').resolve()):
                raise ValueError('Unsafe support path')
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(overlay.read(entry))
    english.install(staging / 'data', english.RUNS / 'downloads/cavestoryen.zip')
    japanese.install(staging / 'data', english.RUNS / 'downloads/dou_1006.zip')
    patches = {}
    for language, relative in [('zh-Hans', 'data'), ('en', 'data/en'), ('jp', 'data/jp')]:
        record = notice.install_language(staging / relative, language)
        if record is None:
            raise ValueError('Expected original script for ' + language)
        patches[relative + '/Stage/Barr.tsc'] = record
    runtimes = runtime_files(arch)
    for runtime in runtimes:
        shutil.copy2(runtime, staging / runtime.name)
    (staging / 'user').mkdir()
    (staging / '启动说明.txt').write_text(
        'CaveStory-rs Windows · 中英日三语本地完整包\n\n'
        '完整解压后双击 CaveStory-rs.exe。默认简体中文，在设置→语言中选择 English 或日本語可切换剧情、物品、地图与图片。\n'
        '请保留原始 Doukutsu.exe、data/en/Doukutsu.exe 和 data/jp/Doukutsu.exe，它们供引擎提取资源。存档和设置位于 user，搬家时一起复制。\n'
        '不含个人或测试存档。建议解压到新目录，勿覆盖已有存档。\n\n'
        '引擎基于 doukutsu-rs，原游戏 Studio Pixel。简体汉化见 Readme.txt；英文 Aeon Genesis，见 data/en/Readme.txt。\n'
        '日文原版说明见 data/jp/Readme.txt。OFL 字体许可见 data/fonts/licenses。附带对应架构的 Microsoft Visual C++ 运行库。\n'
        '仅按用户要求供本地使用，游戏资源不进入源码或默认公开发行包。\n', encoding='utf-8-sig')
    manifest = {'chinese_source': 'https://www.cavestory.one/downloads/dou_sc102.zip',
                'chinese_sha256': HANS_HASH, 'english_source': english.URL, 'english_sha256': english.SHA256,
                'japanese_source': japanese.URL, 'japanese_sha256': japanese.SHA256,
                'build_date': date, 'arch': arch, 'engine_sha256': sha(engine),
                'runtime_files': {str(runtime): {'name': runtime.name, 'sha256': sha(runtime)} for runtime in runtimes}, 'original_chinese_files': originals,
                'personal_saves_included': False, 'distribution_script_patches': patches}
    runtime_source = runtimes[0].parent / 'source-manifest.json'
    if runtime_source.is_file():
        manifest['runtime_source'] = json.loads(runtime_source.read_text(encoding='utf-8'))
    (output / 'source-manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')
    shutil.copy2(output / 'source-manifest.json', staging / 'source-manifest.json')
    target_zip = output / (staging.name + '.zip')
    with zipfile.ZipFile(target_zip, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as target:
        for path in sorted(staging.rglob('*')):
            target.write(path, Path(staging.name) / path.relative_to(staging))
    with zipfile.ZipFile(target_zip) as result:
        if result.testzip() is not None:
            raise ValueError('Package CRC failure')
        for name, digest in originals.items():
            if name.replace('\\', '/') in patches:
                digest = patches[name.replace('\\', '/')]['after_sha256']
            if hashlib.sha256(result.read(staging.name + '/' + name)).hexdigest() != digest:
                raise ValueError('Original resource mismatch: ' + name)
        for name in result.namelist():
            leaf = PurePosixPath(name).name.lower()
            if leaf.startswith('profile') or leaf in ('settings.json', 'config.dat'):
                raise ValueError('Player state in package')
    (output / 'SHA256SUMS').write_text(sha(target_zip) + '  ' + target_zip.name + '\n', encoding='utf-8')
    print(target_zip)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--arch', choices=MACHINES)
    parser.add_argument('--date')
    parser.add_argument('--android-assets', type=Path)
    args = parser.parse_args()
    if args.android_assets:
        prepare_archives(args.android_assets)
    else:
        if not all((args.engine, args.output, args.arch, args.date)):
            parser.error('--engine, --output, --arch and --date are required')
        package(args.engine, args.output, args.arch, args.date)
