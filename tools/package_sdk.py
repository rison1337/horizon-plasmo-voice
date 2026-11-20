#!/usr/bin/env python3
"""Package the flat SDK project with the official cargo-horizon-mod checker."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tool", type=Path, default=Path("cargo-horizon-mod"))
    args = parser.parse_args()
    tool = str(args.tool.resolve()) if args.tool.exists() else str(args.tool)
    manifest = tomllib.loads((ROOT / "mod/horizon-mod.toml").read_text())
    subprocess.run([tool, "horizon-mod", "build"], cwd=ROOT / "mod", check=True)
    original = ROOT / "mod/target/horizon-mod/plasmo_voice.hmod"
    subprocess.run([tool, "horizon-mod", "check", str(original)], check=True)
    subprocess.run(["python3", str(ROOT / "tools/check_source_bundle.py"), str(original)], check=True)
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    target = dist / f"plasmo-voice-{manifest['version']}.hmod"
    shutil.copyfile(original, target)
    checksum = hashlib.sha256(target.read_bytes()).hexdigest()
    target.with_suffix(".hmod.sha256").write_text(f"{checksum}  {target.name}\n")
    print(json.dumps({"artifact": str(target), "sha256": checksum,
                      "bytes": target.stat().st_size, "sdk": manifest["native"]["horizon"]}, indent=2))


if __name__ == "__main__":
    main()
