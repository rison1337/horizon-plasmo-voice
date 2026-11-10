#!/usr/bin/env python3
"""Check the real bundle application, locked dependencies and trait adapter.

Uses the provided public Horizon/Mixins tools without patching the core or
starting any game. Work directories must be new and stay under .local/.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)


def downloaded(path):
    return {(p['name'], p['version'], p['source'], p.get('checksum'))
            for p in tomllib.loads(path.read_text())['package'] if 'source' in p}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--mixins', type=Path, required=True)
    parser.add_argument('--mod-tool', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    work = args.work.resolve()
    if not work.is_relative_to(ROOT / '.local') or work.exists():
        parser.error('--work must be a new directory under .local/')
    work.mkdir(parents=True)
    source = args.source.resolve()
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    bundle = work / 'incoming/plasmo_voice'
    artifact = work / 'plasmo-voice.hnmod'
    run('python3', ROOT / 'tools/package_native_api.py', '--source', source,
        '--stage', bundle, '--output', artifact)
    run(args.mod_tool.resolve(), 'horizon-mod', 'check', artifact)
    run('python3', ROOT / 'tools/check_source_bundle.py', artifact)
    # The published Git source has a documentation symlink; release-source
    # inputs omit symlinks too. Preserve every ordinary tracked file verbatim.
    pristine = work / 'source'
    pristine.mkdir()
    with tempfile.TemporaryFile() as archive:
        run('git', '-C', source, 'archive', 'HEAD', stdout=archive)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as tar:
            for member in tar:
                if member.issym() or member.islnk():
                    continue
                if not (member.isdir() or member.isfile()):
                    raise ValueError(f'non-regular source member: {member.name}')
                tar.extract(member, pristine, filter='data')
    (pristine / '.horizon-revision').write_text(revision + '\n')
    manifest = bundle / 'horizon-mixin.toml'
    # Exactly the launcher-only table that native-compat strips before apply.
    manifest.write_text(manifest.read_text().replace('[api]\npath = "api"\n\n', ''))
    applied = work / 'applied'
    run(args.mixins.resolve(), 'apply', '--workspace', pristine, '--mods', bundle.parent, '--out', applied)
    expected = downloaded(pristine / 'Cargo.lock')
    if downloaded(applied / 'Cargo.lock') != expected:
        raise ValueError('bundle application changed Horizon registry packages')
    # Stage the API crate where the launcher stages it. The sole standalone
    # engine crate has already been validated and staged by horizon-mixins.
    shutil.copytree(bundle / 'api', applied / 'native-api/plasmo_voice')
    manifest = applied / 'Cargo.toml'
    manifest.write_text(manifest.read_text().replace(
        'members = [', 'members = [\n    "native-api/plasmo_voice",', 1))
    # As in the launcher, populate every target's already locked dependencies
    # before offline metadata; this never adds a registry package to the lock.
    run('cargo', 'fetch', '--locked', cwd=pristine)
    run('cargo', 'metadata', '--offline', '--format-version', '1',
        cwd=applied, stdout=subprocess.DEVNULL)
    if downloaded(applied / 'Cargo.lock') != expected:
        raise ValueError('trait staging changed Horizon registry packages')
    run('cargo', 'check', '--locked', '-p', 'horizon-plasmo-native', '--all-targets', cwd=applied)
    run('cargo', 'run', '--locked', '-p', 'horizon-plasmo-engine',
        '--bin', 'plasmo-voice-runtime', '--', '--check', cwd=applied)
    disabled = os.environ.copy()
    disabled['CMAKE'] = str(work / 'missing-cmake')
    run('cargo', 'test', '--locked', '-p', 'horizon-plasmo-native', '--lib',
        'disabled_build_explains_reason_without_registering_voice', cwd=applied, env=disabled)
    result = {'horizon': revision, 'downloaded_packages_unchanged': len(expected),
              'bundle_application': 'pass', 'trait_adapter_check': 'pass',
              'packaged_codec_dsp': 'pass', 'disabled_trait_message': 'pass'}
    (work / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
