#!/usr/bin/env python3
"""Exercise two isolated SDK clients on a local Plasmo Voice server.

Both clients must run with control sockets, HORIZON_PLASMO_DIAGNOSTICS=1 and
explicit virtual audio routes. This driver never launches a physical device or
changes the system default route. Paper must grant these test players /tp and
/fill permission. Images and JSON are evidence, never a pass inferred from source.
"""
import argparse
import json
import math
from pathlib import Path
import struct
import subprocess
import time


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--ctl', type=Path, required=True)
    p.add_argument('--a', type=Path, required=True, help='isolated game folder A')
    p.add_argument('--b', type=Path, required=True, help='isolated game folder B')
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--input-a', default='pv_sdk_in_a')
    p.add_argument('--output-b', default='pv_sdk_out_b.monitor')
    p.add_argument('--tone', type=Path, required=True)
    args = p.parse_args()
    assert args.input_a.startswith('pv_sdk_') and args.output_b.startswith('pv_sdk_')
    args.out.mkdir(parents=True, exist_ok=True)
    folders = [args.a.resolve(), args.b.resolve()]
    results = {}

    def ctl(n, *words):
        cmd = [str(args.ctl.resolve()), '--timeout', '15', '--socket',
               str(folders[n] / 'control.sock'), *map(str, words)]
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        data = json.loads(result.stdout)
        if 'error' in data:
            raise RuntimeError(data['error'])
        return data

    def frame(n=0, name='frame'):
        # GUI action queues drain at the next client tick, not every GPU frame.
        time.sleep(0.1)
        return ctl(n, 'screenshot', (args.out / f'{name}.png').resolve())

    def diag(n):
        return json.loads((folders[n] / 'config/plasmo_voice/diagnostics.json').read_text())

    def settings(n=0):
        return json.loads((folders[n] / 'config/plasmo_voice/settings.json').read_text())

    def tab(n, name):
        widget = next(w for w in ctl(n, 'ui')['widgets'] if w['id'] == name)
        if widget['enabled']:
            ctl(n, 'click', name)
            frame(n)
        return ctl(n, 'ui')['widgets']

    # No fake worker snapshots: wait through real input, frames and device queues.
    ctl(0, 'command', 'kill @e[type=!minecraft:player]')
    for n in (0, 1):
        ctl(n, 'key', 'key.keyboard.left.alt', 'up')
        ctl(n, 'close')
        ctl(n, 'command', f'tp {0 if n == 0 else 3} -60 0')
        ctl(n, 'look', -90 if n == 0 else 90, 0)
    ctl(0, 'key', 'v', 'tap')
    frame()
    results['tabs'] = {name: len(tab(0, name)) for name in
                       ['devices', 'volume', 'activation', 'overlay', 'advanced', 'hotkeys']}
    assert all(count > 8 for count in results['tabs'].values()), results

    ctl(0, 'click', 'm')
    frame()
    ctl(0, 'key', 'k', 'tap')
    frame()
    assert any(w['label'] == 'K' for w in ctl(0, 'ui')['widgets'])
    binding = json.loads((folders[0] / 'config/plasmo_voice/keys.json').read_text())
    assert binding['mute'] == 'key.keyboard.k'
    ctl(0, 'click', 'reset')
    frame()
    assert any(w['label'] == 'M' for w in ctl(0, 'ui')['widgets'])
    results['key_capture_save_reset'] = True

    tab(0, 'devices')
    ctl(0, 'click', 'noise-suppression')
    frame()
    first = settings()['noise_suppression']
    ctl(0, 'click', 'noise-suppression')
    frame()
    assert settings()['noise_suppression'] != first
    results['setting_save'] = True
    # Restore suppression off for the deterministic sine-wave check.
    if settings()['noise_suppression']:
        ctl(0, 'click', 'noise-suppression')
        frame()

    input_process = subprocess.Popen(['paplay', f'--device={args.input_a}', str(args.tone)])
    try:
        evidence = {}
        for screen, command in [('voice_menu', None), ('pause', ('pause',)),
                                ('options', ('options',)), ('inventory', ('open-inventory',)),
                                ('playing', ('close',))]:
            if command:
                ctl(0, 'close')
                ctl(0, *command)
            before = diag(0)['sent']
            ctl(0, 'key', 'key.keyboard.left.alt', 'down')
            time.sleep(1.2)
            snap = diag(0)
            assert snap['ptt_held'] and snap['transmitting'] and snap['sent'] > before, (screen, snap)
            frame(0, 'ptt-' + screen)
            ctl(0, 'key', 'key.keyboard.left.alt', 'up')
            time.sleep(0.6)
            assert not diag(0)['ptt_held']
            evidence[screen] = {'packets': snap['sent'] - before, 'microphone_open': snap['microphone_open']}
        results['ptt_over_interfaces'] = evidence

        # Capture the real receiving output. A synthetic input reaching the
        # output proves capture -> Opus/AES/UDP/server -> decode/mix -> playback.
        ctl(0, 'key', 'key.keyboard.left.alt', 'down')
        raw = args.out / 'received.raw'
        with raw.open('wb') as output:
            subprocess.run(['timeout', '3', 'parec', f'--device={args.output_b}', '--raw',
                            '--format=s16le', '--rate=48000', '--channels=2', '--latency-msec=20'], stdout=output)
        frame(1, 'speaking-player')
        pcm = raw.read_bytes()
        samples = struct.unpack('<' + 'h' * (len(pcm) // 2), pcm)
        rms = math.sqrt(sum(x * x for x in samples) / max(1, len(samples)))
        assert rms > 50, rms
        results['real_output'] = {'samples': len(samples), 'rms_s16': rms,
                                  'receiver': {k: diag(1)[k] for k in ['received', 'output_level', 'error']}}
        raw.unlink()
        ctl(0, 'key', 'key.keyboard.left.alt', 'up')

        # The remote-player handle, picking and persistence all participate.
        target = ctl(0, 'state')['player']['position']
        ctl(1, 'look-at', target[0], target[1] + 1.3, target[2])
        ctl(1, 'mouse', 'right', 'down')
        ctl(1, 'key', 'shift', 'down')
        ctl(1, 'scroll-at', 240, 135, 1)
        time.sleep(0.2)
        ctl(1, 'key', 'shift', 'up')
        ctl(1, 'mouse', 'right', 'up')
        frame(1, 'player-volume-changed')
        volumes = settings(1)['player_volumes']
        assert any(abs(v - 1.05) < 1e-4 for v in volumes.values()), volumes
        results['shift_action_wheel'] = volumes

        # Test wall construction on this throwaway world only; retain frames
        # for visual depth inspection. Do not infer occlusion from packet stats.
        ctl(0, 'key', 'key.keyboard.left.alt', 'down')
        time.sleep(0.3)
        frame(1, 'icon-clear')
        ctl(0, 'command', 'fill 2 -60 -1 2 -56 1 minecraft:stone')
        time.sleep(0.4)
        frame(1, 'icon-wall')
        ctl(0, 'command', 'fill 2 -60 -1 2 -56 1 minecraft:air')
        ctl(0, 'key', 'key.keyboard.left.alt', 'up')
        results['wall_frames_recorded'] = True
    finally:
        ctl(0, 'key', 'key.keyboard.left.alt', 'up')
        input_process.terminate()
        input_process.wait(timeout=5)
        (args.out / 'report.json').write_text(json.dumps(results, indent=2) + '\n')
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
