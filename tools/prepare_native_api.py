#!/usr/bin/env python3
"""Stage the API-only voice mod on an unchanged, public Horizon commit.

The core is a detached Git worktree. Only the development workspace membership,
mod sources and private runtime data are added; no core patch is applied.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--horizon', type=Path, default=ROOT.parent/'horizon')
p.add_argument('--revision', default='HEAD',
               help='public Horizon revision containing both accepted PRs (default: checkout HEAD)')
p.add_argument('--out', type=Path, default=ROOT/'.local/native-api-accepted')
p.add_argument('--refresh-adapter', action='store_true')
a = p.parse_args()
out = a.out.resolve()
if not out.is_relative_to(ROOT/'.local'):
    p.error("--out must be under this project's .local directory")
if not a.refresh_adapter:
    if out.exists():
        p.error('use a new output directory, or --refresh-adapter')
    subprocess.run(['git', '-C', str(a.horizon), 'worktree', 'add', '--detach', str(out),
                    a.revision], check=True)
elif not (out/'.git').is_file():
    p.error('--refresh-adapter requires the previously staged public worktree')
revision = subprocess.check_output(['git', '-C', str(out), 'rev-parse', 'HEAD'], text=True).strip()
(out/'.horizon-revision').write_text(revision+'\n')
(ROOT/'.local/native-api-accepted.json').write_text(json.dumps({
    'source': str(out), 'revision': revision, 'gui_pr': 9556, 'services_pr': 9601,
    'core_patches': False, 'older_servers': 'explicit HORIZON_OLDER_SERVERS=1 only'
}, indent=2))

api=out/'native-api/plasmo_voice';api.mkdir(parents=True,exist_ok=True)
for name in ['src','data']:
    shutil.copytree(ROOT/'native-api'/name,api/name,dirs_exist_ok=True)
manifest=(ROOT/'native-api/Cargo.toml').read_text()
manifest=manifest.replace('../../horizon/crates/horizon-client','../../crates/horizon-client')
manifest=manifest.replace('../../horizon/crates/horizon-protocol','../../crates/horizon-protocol')
manifest=manifest.replace('path = "../engine"','path = "engine"')
(api/'Cargo.toml').write_text(manifest)
shutil.copytree(ROOT/'engine/src',api/'engine/src',dirs_exist_ok=True)
shutil.copyfile(ROOT/'engine/Cargo.toml',api/'engine/Cargo.toml')
wrapper=out/'crates/plasmo-native-preview';(wrapper/'src').mkdir(parents=True,exist_ok=True)
(wrapper/'Cargo.toml').write_text('''[package]
name = "plasmo-native-preview"
version = "0.0.0"
edition = "2024"
[dependencies]
horizon-client = {path="../horizon-client"}
horizon-threads = {path="../horizon-threads"}
horizon-mods = {path="../horizon-mods",default-features=false}
horizon-plasmo-native = {path="../../native-api/plasmo_voice"}
[[bin]]
name = "horizon-client"
path = "src/main.rs"
''')
(wrapper/'src/main.rs').write_text('''#[global_allocator]
static ALLOCATOR: horizon_threads::Allocator = horizon_threads::ALLOCATOR;
static API: [u8; horizon_mods::api_level::MARK.len()] = horizon_mods::api_level::MARK;
fn main() {
    std::hint::black_box(&API);
    horizon_client::mod_api::name_mods(&["plasmo_voice"]);
    horizon_client::entry::main_with(vec![horizon_plasmo_native::hooks()].into());
}
''')
# Translations and checked native libraries, installed later through the normal post-build step.
shutil.copytree(ROOT/'data',out/'native-data/plasmo_voice',dirs_exist_ok=True)
cargo=out/'Cargo.toml';text=cargo.read_text()
for member in ['native-api/plasmo_voice','crates/plasmo-native-preview']:
    if '"'+member+'"' not in text:text=text.replace('members = [','members = [\n    "'+member+'",',1)
cargo.write_text(text)
print(out)
