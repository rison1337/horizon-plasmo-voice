#!/usr/bin/env python3
"""Check adapter regressions against API source, without rewriting registry pins.

The real launcher SDK build is a separate required validation. Ordinary source
compilation alone is not proof of prebuilt SDK ABI compatibility.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, default=ROOT / ".local/adapter-tests")
    parser.add_argument("--clippy", action="store_true")
    parser.add_argument("--interop", action="store_true", help="also check key exchange using OpenSSL")
    args = parser.parse_args()
    work = args.work.resolve()
    if not work.is_relative_to(ROOT / ".local"):
        parser.error("work must be under this project's .local/")
    work.mkdir(parents=True, exist_ok=True)
    for name in ("src", "assets", ".horizon"):
        source = ROOT / "mod" / name
        if source.exists():
            shutil.copytree(source, work / name, dirs_exist_ok=True)
    manifest = (ROOT / "mod/Cargo.toml").read_text() + "\n[workspace]\n\n[profile.dev]\ndebug=0\n"
    (work / "Cargo.toml").write_text(manifest)
    command = ["cargo", "clippy" if args.clippy else "test", "--manifest-path", str(work / "Cargo.toml"), "--lib"]
    if args.clippy:
        command += ["--", "-D", "warnings"]
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / ".local/adapter-test-target"))
    subprocess.run(command, env=environment, check=True)
    if args.interop:
        subprocess.run(["cargo", "test", "--manifest-path", str(work / "Cargo.toml"),
                        "--lib", "openssl_pkcs1v15_key_exchange", "--", "--ignored"],
                       env=environment, check=True)


if __name__ == "__main__":
    main()
