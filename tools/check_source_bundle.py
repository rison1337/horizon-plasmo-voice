#!/usr/bin/env python3
"""Inspect a native artifact without executing it; reject executable payloads.

Checks the complete outer ZIP and all source tar members, including magic
bytes, paths, size limits and the published source hashes. Emits a JSON record.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile
import zipfile


def executable(name, head):
    return (name.lower().endswith(('.dll', '.exe', '.so', '.dylib', '.o', '.obj', '.a', '.lib', '.wasm'))
            or '.so.' in name.lower()
            or head.startswith((b'\x7fELF', b'MZ', b'\0asm', b'!<arch>\n',
                                b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe')))


def inspect(path):
    prefix = 'mixins/plasmo_voice/'
    count = 0
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError('duplicate ZIP entries')
        if archive.testzip() is not None:
            raise ValueError('ZIP checksum failure')
        for name in names:
            if executable(name, archive.read(name)[:8]):
                raise ValueError(f'prebuilt executable payload: {name}')
        manifest = json.loads(archive.read(prefix + 'runtime/data/sources.json'))
        sources = []
        for source in manifest['sources']:
            filename = prefix + 'runtime/data/sources/' + source['archive']
            data = archive.read(filename)
            digest = hashlib.sha256(data).hexdigest()
            if digest != source['sha256']:
                raise ValueError(f'source archive hash mismatch: {filename}')
            size = 0
            with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as tar:
                for member in tar:
                    parts = PurePosixPath(member.name).parts
                    if (member.name.startswith('/') or '..' in parts or '\\' in member.name
                            or not parts or parts[0] != source['archive'].removesuffix('.tar.gz')):
                        raise ValueError(f'unsafe source member: {member.name}')
                    if not (member.isfile() or member.isdir()):
                        raise ValueError(f'non-regular source member: {member.name}')
                    size += member.size
                    if size > 256 * 1024 * 1024:
                        raise ValueError('source expansion limit')
                    if member.isfile():
                        if executable(member.name, tar.extractfile(member).read(8)):
                            raise ValueError(f'prebuilt executable in source: {member.name}')
                        count += 1
            sources.append({'archive': source['archive'], 'sha256': digest, 'expanded_bytes': size})
        return {'prebuilt_executable_payloads': 0, 'source_files': count,
                'sources': sources, 'bytes': path.stat().st_size,
                'artifact_sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    args = parser.parse_args()
    print(json.dumps(inspect(args.bundle), indent=2))


if __name__ == '__main__':
    main()
