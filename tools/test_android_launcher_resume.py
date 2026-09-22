"""Verify a running Android game survives Home -> launcher re-entry.

Start a disposable gameplay session first. This test does not save, install,
clear data or force-stop; a broken launcher can discard unsaved progress.
Evidence must be written outside the source tree, under CaveStory-rs-runs.
"""
import argparse
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adb', default='D:/Tools/adb-fastboot/adb.exe')
    parser.add_argument('--serial', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    runs = Path(__file__).resolve().parents[2] / 'CaveStory-rs-runs'
    output = args.output.resolve()
    if not output.is_relative_to(runs.resolve()):
        parser.error('--output must be inside CaveStory-rs-runs')
    output.mkdir(parents=True, exist_ok=False)
    adb = [args.adb, '-s', args.serial]

    def call(*command):
        return subprocess.check_output(adb + list(command))

    def shell(command):
        return call('shell', command).decode('utf-8', errors='replace')

    package = 'io.github.cavestory_rs'
    before = shell('dumpsys activity activities')
    assert any(package + '/.GameActivity' in line and 'ResumedActivity' in line
               for line in before.splitlines()), 'GameActivity must be foreground'
    result = {'pid_before': shell('pidof ' + package).strip()}
    (output / 'before.png').write_bytes(call('exec-out', 'screencap -p'))
    shell('input keyevent KEYCODE_HOME')
    time.sleep(3)
    result['pid_background'] = shell('pidof ' + package).strip()
    result['launch'] = shell('am start -a android.intent.action.MAIN '
                            '-c android.intent.category.LAUNCHER -f 0x10200000 '
                            '-n ' + package + '/.MainActivity')
    time.sleep(5)
    result['pid_after'] = shell('pidof ' + package).strip()
    after = shell('dumpsys activity activities')
    result['game_resumed'] = any(package + '/.GameActivity' in line and
                                 'ResumedActivity' in line for line in after.splitlines())
    result['passed'] = (result['pid_before'] == result['pid_background'] ==
                        result['pid_after'] and result['game_resumed'])
    (output / 'after.png').write_bytes(call('exec-out', 'screencap -p'))
    (output / 'activities.txt').write_text(after, encoding='utf-8')
    (output / 'logcat.txt').write_text(shell(
        'logcat -d -v threadtime -s ActivityTaskManager ActivityManager SDL libc AndroidRuntime'),
        encoding='utf-8')
    (output / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps(result))
    assert result['passed'], 'Launcher re-entry destroyed or failed to resume the running game'


if __name__ == '__main__':
    main()
