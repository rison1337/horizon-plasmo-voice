#!/usr/bin/env python3
"""Export the unchanged brand illustration at UI sizes and lay out its banner.

Requires ImageMagick and librsvg (rsvg-convert). No game textures or fonts are
bundled. The wordmark uses the original small bitmap alphabet below.
"""
import base64
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
FONT = {
    "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
    "C": ["01111", "10000", "10000", "10000", "10000", "10000", "01111"],
    "D": ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
    "E": ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
    "F": ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
    "H": ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
    "I": ["11111", "00100", "00100", "00100", "00100", "00100", "11111"],
    "L": ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
    "M": ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
    "N": ["10001", "11001", "11001", "10101", "10011", "10011", "10001"],
    "O": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
    "P": ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
    "R": ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
    "S": ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
    "T": ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
    "U": ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
    "V": ["10001", "10001", "10001", "10001", "10001", "01010", "00100"],
    "W": ["10001", "10001", "10001", "10101", "10101", "10101", "01010"],
    "X": ["10001", "10001", "01010", "00100", "01010", "10001", "10001"],
    "Y": ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
    "Z": ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
    "+": ["00000", "00100", "00100", "11111", "00100", "00100", "00000"],
    ".": ["00000", "00000", "00000", "00000", "00000", "00110", "00110"],
    " ": ["00000"] * 7,
}


def text(label, x, y, scale, color):
    cells = []
    for index, letter in enumerate(label):
        for row, bits in enumerate(FONT[letter]):
            for column, bit in enumerate(bits):
                if bit == "1":
                    cells.append(f'<rect x="{x + (index * 6 + column) * scale}" '
                                 f'y="{y + row * scale}" width="{scale}" height="{scale}"/>')
    return f'<g fill="{color}">' + "".join(cells) + "</g>"


def main():
    magick = shutil.which("magick")
    rasterizer = shutil.which("rsvg-convert")
    if not magick or not rasterizer:
        raise SystemExit("Install ImageMagick and librsvg's rsvg-convert first")
    source = ASSETS / "icon-source.png"
    # Technical size exports preserve the supplied illustration and alpha.
    # A 64-pixel base keeps the enlarged launcher/README exports on one grid.
    subprocess.run([magick, str(source), "-filter", "point", "-resize", "64x64",
                    "-background", "none", "-gravity", "center", "-extent", "64x64",
                    "-strip", str(ASSETS / "icon-64.png")], check=True)
    for size in [32, 40, 128, 256, 512]:
        subprocess.run([magick, str(ASSETS / "icon-64.png"), "-filter", "point",
                        "-resize", f"{size}x{size}", "-strip",
                        str(ASSETS / f"icon-{size}.png")], check=True)

    icon = base64.b64encode((ASSETS / "icon-256.png").read_bytes()).decode("ascii")
    svg = ['<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="640" '
           'viewBox="0 0 1280 640" shape-rendering="crispEdges">',
           '<rect width="1280" height="640" fill="#111a1e"/>',
           '<path d="M40 104V40H104 M1176 40H1240V104 M40 536V600H104 '
           'M1176 600H1240V536" fill="none" stroke="#334947" stroke-width="8"/>',
           '<path d="M56 64H120 M56 64V120" fill="none" stroke="#79e38d" stroke-width="8"/>',
           '<rect x="72" y="116" width="328" height="408" fill="#19262a"/>',
           '<path d="M72 116H400V524H72Z" fill="none" stroke="#2a3c3e" stroke-width="4"/>',
           f'<image href="data:image/png;base64,{icon}" x="76" y="160" '
           'width="320" height="320" style="image-rendering:pixelated"/>']
    svg += [text("PLASMO VOICE", 456, 202, 8, "#071013"),
            text("PLASMO VOICE", 452, 196, 8, "#fff0ce"),
            text("FOR HORIZON", 452, 290, 6, "#79e38d"),
            '<rect x="452" y="374" width="88" height="6" fill="#79e38d"/>',
            text("PROXIMITY VOICE CHAT", 452, 414, 3, "#bdceca"),
            text("NATIVE RUST . LINUX + WINDOWS", 452, 466, 2, "#879d98"),
            '</svg>']
    banner = ASSETS / "github-social.svg"
    banner.write_text("\n".join(svg))
    subprocess.run([rasterizer, "-o", str(ASSETS / "github-social.png"), str(banner)], check=True)
    destination = ROOT / "publishing/horizon-mods/icons/plasmo_voice.png"
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ASSETS / "icon-256.png", destination)
    if destination.stat().st_size > 256 * 1024:
        raise SystemExit("Registry icon exceeds 256 KiB")
    print("Exported transparent PNG icons, 1280x640 GitHub preview and registry icon")


if __name__ == "__main__":
    main()
