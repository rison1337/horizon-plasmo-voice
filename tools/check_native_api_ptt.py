#!/usr/bin/env python3
"""Exercise physical PTT input and final GUI indication with all audio disabled."""
import json
import os
from pathlib import Path
import socket
import subprocess
import time

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
run = json.loads((ROOT / '.local/latest-run.json').read_text())
assert not run['audio']
environment = dict(v.split(b'=', 1) for v in
                   (Path('/proc') / str(run['pid']) / 'environ').read_bytes().split(b'\0') if b'=' in v)
assert environment.get(b'HORIZON_NO_AUDIO') == b'1'
data = Path(run['home']) / 'horizon-mods/.horizon-mod-data/plasmo_voice'
out = Path(run['scratch']) / 'native-api-ptt'
out.mkdir(exist_ok=True)
x11 = ROOT / '.local/reference-original/x11'
env = dict(os.environ, DISPLAY=':91', LD_LIBRARY_PATH=str(x11 / 'usr/lib'))


def command(line):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
        stream.settimeout(30)
        stream.connect(run['control'])
        stream.sendall((line + '\n').encode())
        reply = json.loads(stream.makefile().readline())
        assert not reply.get('error'), reply
        return reply


def xd(*args):
    return subprocess.check_output([str(x11 / 'usr/bin/xdotool'), *args], env=env, text=True)


def wait(predicate, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            diagnostic = json.loads((data / 'diagnostics.json').read_text())
            if predicate(diagnostic):
                return diagnostic
        except (FileNotFoundError, json.JSONDecodeError):
            pass
        time.sleep(.07)
    raise AssertionError(json.loads((data / 'diagnostics.json').read_text()))


def playing():
    for _ in range(10):
        if command('ui')['screen'] == 'playing':
            return
        command('key escape tap')
        time.sleep(.1)
    raise AssertionError(command('ui'))


def capture(name):
    path = out / (name + '.png')
    command('screenshot ' + str(path))
    return Image.open(path).convert('RGB').crop((612, 580, 668, 718))


native = xd('search', '--pid', str(run['pid'])).splitlines()[-1]
reference = json.loads((ROOT / '.local/native-api-reference.json').read_text())
other = xd('search', '--pid', str(reference['pid'])).splitlines()[-1]
xd('windowraise', native, 'windowfocus', native)
wait(lambda d: d['input_focused'] and d['status'] == 'Connected')
results = {}
try:
    for name, opening in [
        ('playing', None), ('pause', 'pause'), ('options', 'options'),
        ('video', 'options video'), ('sound', 'options sound'),
        ('inventory', 'open-inventory'), ('mod', 'key v tap'), ('chat', 'key t tap'),
    ]:
        playing()
        if opening:
            command(opening)
        time.sleep(.25)
        screen = command('ui')['screen']
        before = capture(name + '-idle')
        xd('keydown', 'Alt_L')
        wait(lambda d: d['ptt_held'] and d['input_active'])
        held = capture(name + '-held')
        changed = sum(1 for pixel in ImageChops.difference(before, held).get_flattened_data()
                      if max(pixel) > 15)
        assert changed > 30, (name, changed)
        xd('keyup', 'Alt_L')
        wait(lambda d: not d['ptt_held'])
        assert command('ui')['screen'] == screen
        results[name] = dict(pressed=True, released=True, changed_icon_pixels=changed)
        print(name + ': physical PTT press/release, final GUI icon visible', flush=True)
    playing()
    xd('keydown', 'Alt_L')
    wait(lambda d: d['ptt_held'])
    command('pause')
    wait(lambda d: d['ptt_held'] and d['input_active'])
    xd('windowraise', other, 'windowfocus', other)
    wait(lambda d: not d['input_focused'] and not d['ptt_held'] and not d['input_active'])
    xd('keyup', 'Alt_L')
    xd('windowraise', native, 'windowfocus', native)
    wait(lambda d: d['input_focused'] and not d['ptt_held'])
    results['focus_loss_clears_hold'] = True
    results['audio_devices_disabled'] = True
finally:
    xd('keyup', 'Alt_L')
    playing()
(ROOT / '.local/native-api-ptt-report.json').write_text(json.dumps(results, indent=2))
print(json.dumps(results, indent=2))
