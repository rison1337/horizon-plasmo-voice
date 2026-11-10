#!/usr/bin/env python3
"""Package the Hooks/Host adapter as a normal, self-contained .hnmod.

Builds pin the public source containing the accepted APIs. A portable release
requires their official hooks number; this script never guesses one.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
VERSION = "0.5.4"
REQUIRED_HOOKS = {"gui_render_post", "client_disconnecting", "client_stopping", "menu_background"}
REQUIRED_HELPERS = {"client_state", "client_entities", "client_heads", "client_profiles",
                    "client_visual_clear", "client_block_obstruction", "gui_paint",
                    "gui_nine_slice", "gui_tooltip", "set_mod_key", "client_action_bar",
                    "text_width", "playing", "window", "player", "send_packet"}


def stage(destination, revision):
    destination.mkdir(parents=True)
    (destination / "horizon-mixin.toml").write_text(f'''[mod]
id = "plasmo_voice"
version = "{VERSION}"

[compatibility]
horizon_revision = "{revision}"

[api]
path = "api"

[[crate]]
id = "engine"
path = "engine"

[[post_build]]
crate = "engine"
bin = "plasmo-voice-runtime"
''')
    api = destination / "api"
    for name in ["src", "data"]:
        shutil.copytree(ROOT / "native-api" / name, api / name)
    manifest = (ROOT / "native-api/Cargo.toml").read_text()
    manifest = manifest.replace("../../horizon/crates/", "../../crates/")
    manifest = manifest.replace('path = "../engine"', 'path = "../../native-crates/plasmo_voice/engine"')
    (api / "Cargo.toml").write_text(manifest)
    # One declared standalone crate owns the engine, source build and installer.
    # Its dependencies are registry requirements from Horizon's existing lock.
    engine = destination / "engine"
    for name in ["src", "data"]:
        shutil.copytree(ROOT / "engine" / name, engine / name)
    for name in ["Cargo.toml", "build.rs", "README.md"]:
        shutil.copyfile(ROOT / "engine" / name, engine / name)
    shutil.copytree(ROOT / "data", destination / "data")
    shutil.copyfile(ROOT / "LICENSE", destination / "LICENSE")
    shutil.copyfile(ROOT / "THIRD_PARTY_NOTICES.md", destination / "NOTICE.md")
    shutil.copyfile(ROOT / "native-hooks/README.md", destination / "README.md")
    shutil.copyfile(ROOT / "native-hooks/VALIDATION.md", destination / "README-validation.md")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True,
                        help="Horizon source with both required client API extensions")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--hooks", type=int,
                      help="official API minimum (default: actual stamped level of tested source)")
    mode.add_argument("--pinned", action="store_true",
                      help="developer check only: pin the public commit while APIs await stamping")
    parser.add_argument("--stage", type=Path, help="also keep an unpacked bundle folder")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    forbidden = [path for path in (ROOT / "data").rglob("*")
                 if path.is_file() and (path.name.endswith((".dll", ".so", ".dylib", ".exe", ".a", ".o"))
                                        or ".so." in path.name)]
    if forbidden:
        parser.error("prebuilt executable payloads are forbidden: " + ", ".join(map(str, forbidden)))
    source = args.source.resolve(strict=True)
    revision = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        parser.error("source needs an actual Git commit")
    catalog = json.loads((source / "native-hooks.json").read_text())
    fragments = source / "tools/native-hook-fragments"
    level_file = source / "crates/horizon-client/src/game/native_hooks.rs"
    level = re.search(r"NATIVE_HOOKS_API[^=]*=\s*(\d+)", level_file.read_text())
    names = {entry["name"] for entry in catalog.get("hooks", [])}
    helpers = {entry["name"] for entry in catalog.get("helpers", [])}
    if not args.pinned:
        # Stamping an uncommitted development checkout does not assign an
        # official number. Test the catalog committed with this source identity.
        api_paths = ["native-hooks.json", "crates/horizon-client/src/game/native_hooks.rs",
                     "tools/native-hook-fragments"]
        if subprocess.run(["git", "-C", str(source), "diff", "--quiet", "HEAD", "--",
                           *api_paths]).returncode:
            parser.error("API catalog must match the committed source; local stamping is not a release")
        if not level:
            parser.error("source has no numeric API level")
        if args.hooks is None:
            args.hooks = 411
        if args.hooks < 409:
            parser.error("minimum API predates send_packet")
        # A release must use the stamped catalog, never pending fragments.
        if REQUIRED_HOOKS - names:
            parser.error("required hooks await official stamping; --pinned is a development check only")
        if REQUIRED_HELPERS - helpers:
            parser.error("required Host services are not in the accepted catalog")
        if not 411 <= args.hooks <= int(level[1]):
            parser.error("minimum must include registered API 411 and be supported by the tested source")
    else:
        for fragment in fragments.glob("*.json"):
            pending = json.loads(fragment.read_text())
            names.update(entry["name"] for entry in pending.get("hooks", []))
            helpers.update(entry["name"] for entry in pending.get("helpers", []))
        if REQUIRED_HOOKS - names:
            parser.error("source is missing required client hooks")
        if REQUIRED_HELPERS - helpers:
            parser.error("source is missing required client Host services")
    suffix = "-pinned-check" if args.pinned else ""
    output = args.output or ROOT / "dist" / f"plasmo-voice-{VERSION}{suffix}.hnmod"
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="plasmo-native-api-") as temporary:
        bundle = Path(temporary) / "plasmo_voice"
        stage(bundle, revision)
        manifest = dict(schema=1, id="plasmo_voice", version=VERSION, game="26.3",
                        horizon=revision, mixins=MIXINS)
        if args.hooks is not None:
            manifest["hooks"] = args.hooks
        with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
            def add(name, data):
                entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                entry.compress_type = zipfile.ZIP_DEFLATED
                entry.external_attr = 0o100644 << 16
                archive.writestr(entry, data)
            add("horizon-native.json", json.dumps(manifest, indent=2).encode())
            for file in sorted(bundle.rglob("*")):
                if file.is_file():
                    add("mixins/plasmo_voice/" + file.relative_to(bundle).as_posix(),
                        file.read_bytes())
        if args.stage:
            if args.stage.exists():
                parser.error("--stage destination must not exist")
            shutil.copytree(bundle, args.stage)
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix(output.suffix + ".sha256").write_text(digest + "  " + output.name + "\n")
    print(json.dumps(dict(archive=str(output), sha256=digest, horizon=revision,
                          hooks=args.hooks, portable=args.hooks is not None), indent=2))


if __name__ == "__main__":
    main()
