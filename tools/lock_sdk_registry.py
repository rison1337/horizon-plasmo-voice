#!/usr/bin/env python3
"""Resolve private registry dependencies while retaining the signed SDK graph.

Only inert Cargo manifests and metadata are used: this never copies or executes
mod sources, build scripts, proc macros, or downloaded code. SDK sources are
excluded from the registry lock, as with horizon-launcher native-lock.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def records(path):
    return tomllib.loads(path.read_text())["package"]


def identity(package):
    return package["name"], package["version"]


def dependencies(package, packages):
    result = []
    for dep in package.get("dependencies", []):
        name, *parts = dep.split()
        version = parts[0] if parts else None
        matches = [p for p in packages if p["name"] == name and (not version or p["version"] == version)]
        if len(matches) != 1:
            raise ValueError(f"ambiguous dependency: {dep}")
        result.append(identity(matches[0]))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk-lock", type=Path, required=True)
    args = parser.parse_args()
    mod = ROOT / "mod"
    manifest = (mod / "Cargo.toml").read_text()
    # This project's only SDK declaration is a one-line pinned API path.
    manifest = "\n".join(line for line in manifest.splitlines() if not line.startswith("horizon-mod-api ="))
    manifest += '\n[lib]\npath="empty.rs"\n[workspace]\n'
    sdk = records(args.sdk_lock)
    with tempfile.TemporaryDirectory(prefix="plasmo-registry-", dir=ROOT / ".local") as temporary:
        directory = Path(temporary)
        (directory / "Cargo.toml").write_text(manifest)
        (directory / "empty.rs").write_text("")
        (directory / "Cargo.lock").write_bytes(args.sdk_lock.read_bytes())
        subprocess.run(["cargo", "metadata", "--manifest-path", str(directory / "Cargo.toml"),
                        "--format-version", "1"], stdout=subprocess.DEVNULL, check=True)
        resolved = records(directory / "Cargo.lock")
    merged = {identity(p): (p, dependencies(p, sdk)) for p in sdk if "source" in p}
    for package in resolved:
        merged.setdefault(identity(package), (package, dependencies(package, resolved)))
    pending = [identity(tomllib.loads((mod / "Cargo.toml").read_text())["package"])]
    closure = set()
    while pending:
        key = pending.pop()
        if key in closure:
            continue
        closure.add(key)
        pending.extend(merged[key][1])
    lines = ["# Exact registry closure preserving the published SDK's dependency edges.", "version = 4", ""]
    for key in sorted(closure):
        package, deps = merged[key]
        lines.append("[[package]]")
        for name in ("name", "version", "source", "checksum"):
            if name in package:
                lines.append(f"{name} = {json.dumps(package[name])}")
        if deps:
            lines += ["dependencies = ["] + [f" {json.dumps(' '.join(dep))}," for dep in deps] + ["]"]
        lines.append("")
    (mod / "Cargo.lock").write_text("\n".join(lines))
    print(f"Pinned {len(closure) - 1} registry packages; SDK edges retained.")


if __name__ == "__main__":
    main()
