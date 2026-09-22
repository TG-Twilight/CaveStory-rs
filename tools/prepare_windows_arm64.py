from pathlib import Path
import json,urllib.request,hashlib,zipfile
import argparse
ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description="Prepare local MSVC 14.29 ARM64 tools from the installed VS catalog, verifying every payload hash.")
parser.add_argument('--catalog',type=Path)
args=parser.parse_args()
catalog=args.catalog
if catalog is None:
    for candidate in Path('C:/ProgramData/Microsoft/VisualStudio/Packages/_Instances').glob('*/catalog.json'):
        contents=json.loads(candidate.read_text(encoding='utf-8-sig'))
        if any(p['id']=='Microsoft.VC.14.29.16.11.Tools.HostX64.TargetARM64.base' for p in contents['packages']):
            catalog=candidate
            break
if catalog is None:raise SystemExit('Install VS 2019 Build Tools, or specify its --catalog path')
packages=json.loads(catalog.read_text(encoding='utf-8-sig'))['packages']
dest=ROOT/'.cache/toolchains/msvc-arm64';dest.mkdir(parents=True,exist_ok=True)
ids=['Microsoft.VC.14.29.16.11.Tools.HostX64.TargetARM64.base','Microsoft.VC.14.29.16.11.CRT.ARM64.Desktop.base','Microsoft.VC.14.29.16.11.CRT.ARM64.Desktop.debug.base']
ids.append('Microsoft.VC.14.29.16.11.CRT.ARM64.Store.base')
records=[]
for id in ids:
    p=next(p for p in packages if p['id']==id)
    for payload in p['payloads']:
        f=dest/payload['fileName']
        if not f.exists():
            with urllib.request.urlopen(payload['url'],timeout=60) as response:
                data=response.read(payload['size']+1)
            assert len(data)==payload['size']
            assert hashlib.sha256(data).hexdigest()==payload['sha256']
            f.write_bytes(data)
        assert hashlib.sha256(f.read_bytes()).hexdigest()==payload['sha256']
        with zipfile.ZipFile(f) as z:
            for name in z.namelist():
                if not name.startswith('Contents/'):continue
                target=dest/name.removeprefix('Contents/')
                assert target.resolve().is_relative_to(dest.resolve())
                if name.endswith('/'):continue
                data=z.read(name)
                if target.exists() and target.read_bytes()==data:continue
                target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data)
        records.append(payload)
        print(id, 'verified and extracted',flush=True)
(dest/'sources.json').write_text(json.dumps(records,indent=2))
