# /// script
# requires-python = ">=3.10"
# dependencies = ["fonttools", "uharfbuzz", "pillow"]
# ///
"""Rebuild reflectapi's logo files.

    uv run docs/graphics/build.py

Writes docs/src/reflectapi-wordmark.svg and reflectapi-wordmark-dark.svg: the
crab from docs/src/reflectapi-logo.png beside "reflectapi" in Oxanium 600,
drawn as paths so the logo looks the same without the font. Then renders
docs/graphics/social-preview.html to social-preview.png with headless Chrome,
found on PATH or through $CHROME; without one, that step is skipped.
"""

import base64
import io
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

import uharfbuzz as hb
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from PIL import Image

HERE = Path(__file__).resolve().parent
SRC = HERE.parent / "src"
CRAB = SRC / "reflectapi-logo.png"
FONT_URL = "https://github.com/google/fonts/raw/main/ofl/oxanium/Oxanium%5Bwght%5D.ttf"

# Crab height, gap between crab and text, text size, letter spacing (em).
CRAB_H, GAP, SIZE, TRACK = 34.0, 6.0, 26.0, -0.02
NIGHT, CLAW, PAPER = "#131414", "#E88542", "#F7F7F7"
TEXT = "reflectapi"
COLORS = ["r"] * 7 + ["a"] * 3  # "reflect" in ink, "api" in orange


def oxanium_600():
    """Oxanium (OFL), fetched from Google Fonts' repository, at weight 600."""
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "reflectapi"
    path = cache / "Oxanium-var.ttf"
    if not path.exists():
        cache.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(FONT_URL, path)
    font = instancer.instantiateVariableFont(TTFont(path), {"wght": 600})
    buf = io.BytesIO()
    font.save(buf)
    return buf.getvalue()


def lettering(data, x0, cy, fills):
    """SVG paths for TEXT starting at x0, centred on cy, and the ink's right edge."""
    font = TTFont(io.BytesIO(data))
    upem = font["head"].unitsPerEm
    hhea = font["hhea"]
    scale = SIZE / upem
    hbfont = hb.Font(hb.Face(data))
    buf = hb.Buffer()
    buf.add_str(TEXT)
    buf.guess_segment_properties()
    hb.shape(hbfont, buf, {"kern": True})

    # Centre the line box on the crab as CSS align-items: center would, with
    # metrics rounded to whole pixels as browsers do.
    asc, desc, gap = (round(v * scale) for v in (hhea.ascent, -hhea.descent, hhea.lineGap))
    baseline = cy + gap / 2 + asc - (asc + desc + gap) / 2

    glyphs = font.getGlyphSet()
    order = font.getGlyphOrder()
    groups = {f: [] for f in fills}
    pen_x, right = 0.0, x0
    for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
        name = order[info.codepoint]
        ox = x0 + (pen_x + pos.x_offset) * scale
        oy = baseline - pos.y_offset * scale
        svg = SVGPathPen(glyphs, ntos=lambda v: ("%.2f" % v).rstrip("0").rstrip("."))
        glyphs[name].draw(TransformPen(svg, (scale, 0, 0, -scale, ox, oy)))
        groups[fills[info.cluster]].append(svg.getCommands())
        bounds = BoundsPen(glyphs)
        glyphs[name].draw(bounds)
        if bounds.bounds:
            right = max(right, ox + bounds.bounds[2] * scale)
        pen_x += pos.x_advance + TRACK * upem
    return [f'<path fill="{f}" d="{"".join(d)}"/>' for f, d in groups.items()], right


def crab_png(height):
    im = Image.open(CRAB).convert("RGBA")
    im = im.resize((round(im.width * height / im.height), height), Image.LANCZOS)
    # 256 colours look the same at logo sizes, at a fifth of the bytes.
    im = im.quantize(256, method=Image.Quantize.FASTOCTREE)
    out = io.BytesIO()
    im.save(out, "PNG", optimize=True)
    return base64.b64encode(out.getvalue()).decode()


def wordmark(data, dark):
    """The light wordmark, or the dark one with the crab on a light tile."""
    with Image.open(CRAB) as im:
        aspect = im.width / im.height
    png = crab_png(160)
    parts = []
    if dark:
        pad = 2.0
        inner = CRAB_H - 2 * pad
        crab_w = inner * aspect + 2 * pad
        parts.append(f'<rect width="{crab_w:.2f}" height="{CRAB_H:g}" rx="6" fill="{PAPER}"/>')
        parts.append(f'<image x="{pad:g}" y="{pad:g}" width="{inner * aspect:.2f}" '
                     f'height="{inner:g}" href="data:image/png;base64,{png}"/>')
    else:
        crab_w = CRAB_H * aspect
        parts.append(f'<image width="{crab_w:.2f}" height="{CRAB_H:g}" '
                     f'href="data:image/png;base64,{png}"/>')
    ink = PAPER if dark else NIGHT
    fills = [ink if c == "r" else CLAW for c in COLORS]
    paths, w = lettering(data, crab_w + GAP, CRAB_H / 2, fills)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.2f} {CRAB_H:g}" '
            f'width="{w:.2f}" height="{CRAB_H:g}" role="img" aria-label="reflectapi">'
            f"<title>reflectapi</title>" + "".join(parts + paths) + "</svg>\n")


def social_preview():
    names = ["chromium", "chromium-browser", "google-chrome", "chrome", "chrome-headless-shell"]
    chrome = os.environ.get("CHROME") or next(filter(None, map(shutil.which, names)), None)
    if not chrome:
        print("skipped social-preview.png: no Chrome found; set $CHROME", file=sys.stderr)
        return
    out = HERE / "social-preview.png"
    with tempfile.TemporaryDirectory() as profile:
        subprocess.run([
            chrome, "--headless", "--no-sandbox", "--hide-scrollbars",
            "--allow-file-access-from-files", f"--user-data-dir={profile}",
            "--window-size=1280,640", "--virtual-time-budget=10000",
            f"--screenshot={out}", (HERE / "social-preview.html").as_uri(),
        ], check=True, capture_output=True)
    print(out.relative_to(HERE.parent.parent))


def main():
    data = oxanium_600()
    for dark, name in [(False, "reflectapi-wordmark.svg"), (True, "reflectapi-wordmark-dark.svg")]:
        (SRC / name).write_text(wordmark(data, dark))
        print((SRC / name).relative_to(HERE.parent.parent))
    social_preview()


if __name__ == "__main__":
    main()
