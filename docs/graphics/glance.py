#!/usr/bin/env python3
"""Draws docs/src/design/glance.svg: the API design standard at a glance, an
annotated request, response and error. The book's standard includes it, and
build-extras.py inlines it into the landing page's preview.

    python3 docs/graphics/glance.py

The lines, notes and layout are the lists and constants below. Tokens are
placed by column, so the arrows land on them whatever the monospace font's
exact width. Keep W just past the longest note: the dialog and the previews
size to the drawing.
"""
from html import escape
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "src" / "design" / "glance.svg"

FS = 14            # code font size
CW = FS * 0.6      # monospace advance
LH = 23            # line height
PX = 22            # panel's inner left padding
PANEL_X = 0
PANEL_W = 470
LABEL_X = 520      # where right-hand notes start
W = 780
TOP = 78           # space above the panel for the first line's notes

# Each line is a list of (text, kind); kind None = plain, "k" = annotated
# token (orange), "m" = muted. A line may carry a right-hand note on a token.
blocks = [
    ("Request", [
        ([("POST", "k"), (" ", None), ("/api/v1", "k"), ("/", None), ("orders.list", "k")], None),
        ([("content-type", "m"), (": application/json", None)], ("JSON, or MessagePack", 0)),
        ([("authorization", "k"), (": Bearer <token>", None)], ("The only header for authentication", 0)),
        ([("traceparent", "k"), (": 00-4bf9…-01", None)], ("Trace context, returned in the response", 0)),
        ([("", None)], None),
        ([('{"page": {"cursor": "eyJ…", "limit": 20}}', "k")], ("Input: always an object in the body", 0)),
    ]),
    ("Response", [
        ([("HTTP/1.1 ", None), ("200", "k"), (" OK", None)], ("200 for every success", 1)),
        ([("{", None)], None),
        ([("  ", None), ('"items"', "k"), (': [{"id": "e7fe…"}],', None)], ("Lists are wrapped in items", 1)),
        ([("  ", None), ('"pagination"', "k"), (': {"next_cursor": "eyJ…"}', None)], ("Next page, or null at the end", 1)),
        ([("}", None)], None),
    ]),
    ("Error", [
        ([("HTTP/1.1 ", None), ("404", "k"), (" Not Found", None)], ("A 4xx, chosen by the error's case", 1)),
        ([('{"not_found": {"id": "e7fe…"}}', "k")], ("A value of the call's error type", 0)),
    ]),
]

# Notes above the first line, pointing down at its three tokens:
# (token index on line 1, text, row from the top)
top_notes = [(0, "Always POST", 0), (2, "Major version", 1), (4, "Call name: resource.verb", 2)]

parts = []
arrows = []
y = TOP + 18
line_ys = []
code_lines = []
for title, lines in blocks:
    y += 8
    parts.append(f'<text class="glance-section" x="{PANEL_X + PANEL_W - 16}" y="{y:.0f}" text-anchor="end">{escape(title.upper())}</text>')
    y += LH
    for tokens, note in lines:
        col = 0
        spans = []
        positions = []
        for text, kind in tokens:
            x = PX + col * CW
            positions.append((x, x + len(text) * CW))
            if text:
                cls = {"k": "glance-key", "m": "glance-dim", None: "glance-code"}[kind]
                spans.append(f'<tspan class="{cls}" x="{x:.1f}">{escape(text)}</tspan>')
            col += len(text)
        parts.append(f'<text class="glance-mono" y="{y:.0f}">{"".join(spans)}</text>')
        code_lines.append((y, positions))
        if note:
            text, token = note
            tx_end = max(e for _, e in positions) + 8
            ay = y - FS * 0.35
            arrows.append(f'<line class="glance-arrow" x1="{LABEL_X - 8}" y1="{ay:.1f}" x2="{tx_end:.1f}" y2="{ay:.1f}" marker-end="url(#glance-head)"/>')
            parts.append(f'<text class="glance-note" x="{LABEL_X}" y="{y:.0f}">{escape(text)}</text>')
        y += LH
    y += 6
panel_h = y - TOP + 4
H = TOP + panel_h + 2

# top notes: rows at 18, 40, 62; vertical arrows down to the token's top
first_y, first_pos = code_lines[0]
for token, text, row in top_notes:
    x0, x1 = first_pos[token]
    ny = 16 + row * 22
    cx = (x0 + x1) / 2
    parts.append(f'<text class="glance-note" x="{x0:.1f}" y="{ny}">{escape(text)}</text>')
    arrows.append(f'<line class="glance-arrow" x1="{cx:.1f}" y1="{ny + 6}" x2="{cx:.1f}" y2="{first_y - FS - 3:.1f}" marker-end="url(#glance-head)"/>')

style = """
.glance { --g-fg: #131414; --g-dim: #6E6F70; --g-note: #494B4D; --g-code: #F7F7F7;
  --g-border: #E0E2E2; --g-accent: #E88542; --g-key: #A8501A;
  display: block; width: 100%; min-width: 640px; height: auto; }
html.coal .glance, html.navy .glance, html.ayu .glance { --g-fg: #F7F7F7; --g-dim: #8B8D8E;
  --g-note: #B2B5B6; --g-code: #1B1C1C; --g-border: #26282A; --g-key: #E88542; }
.alt .glance { --g-code: #FFFFFF; }
.glance-panel { fill: var(--g-code); stroke: var(--g-border); }
.glance-mono { font-family: "Geist Mono", ui-monospace, "Courier New", monospace; font-size: 14px; }
.glance-code { fill: var(--g-fg); }
.glance-dim { fill: var(--g-dim); }
.glance-key { fill: var(--g-key); font-weight: 500; }
.glance-section { font-family: "Geist Mono", ui-monospace, monospace; font-size: 11px;
  letter-spacing: .08em; fill: var(--g-dim); }
.glance-note { font-family: "Geist", "Helvetica Neue", Arial, sans-serif; font-size: 14px; fill: var(--g-note); }
.glance-arrow { stroke: var(--g-accent); stroke-width: 1.25; }
.glance-head { fill: var(--g-accent); }
""".strip()

svg = "\n".join([
    f'<svg class="glance" viewBox="0 0 {W} {H:.0f}" width="{W}" height="{H:.0f}" role="img" aria-labelledby="glance-title glance-desc" xmlns="http://www.w3.org/2000/svg">',
    '<title id="glance-title">The API design standard at a glance</title>',
    '<desc id="glance-desc">A request: POST /api/v1/orders.list, always POST, with the major version and a call name of resource and verb; '
    'content-type JSON or MessagePack; authorization, the only header for authentication; traceparent, returned in the response; '
    'and the input, always an object in the body. A response: 200 for every success, with the list wrapped in items and pagination '
    'holding the next cursor, null at the end. An error: a 4xx status chosen by the error case, and a body that is the call\'s typed error value, such as not_found with the id.</desc>',
    f'<style>{style}</style>',
    '<defs><marker id="glance-head" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">'
    '<path class="glance-head" d="M0,0 L8,4 L0,8 z"/></marker></defs>',
    f'<rect class="glance-panel" x="{PANEL_X + 0.5}" y="{TOP + 0.5}" width="{PANEL_W}" height="{panel_h:.0f}" rx="8"/>',
    *arrows,
    *parts,
    "</svg>",
])
assert "\n\n" not in svg
OUT.write_text(svg + "\n")
print(f"wrote {OUT.relative_to(Path.cwd()) if OUT.is_relative_to(Path.cwd()) else OUT} ({W}x{H:.0f})")
