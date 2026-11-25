#!/usr/bin/env python3
"""Finish the built book: the landing page, and the files search engines and
coding agents look for.

Run after `mdbook build`, from anywhere: `python3 docs/build-extras.py`, or
`python3 docs/build-extras.py DIR` for a book built with `mdbook build -d DIR`.

- `index.html`: the landing page from `docs/landing/`, in place of mdBook's
  copy of the first chapter, which stays at `introduction.html`, with the
  standard's annotated example from `src/design/glance.svg` inlined.
- `sitemap.xml`: every page, for search engines.
- `llms.txt`: the guide's table of contents with links (https://llmstxt.org).
- `llms-full.txt`: the whole guide as one Markdown file, in reading order,
  so an agent can read all of it with one fetch.

It first checks the form of the API design standard's rules, and stops if
one is malformed: see `check_standard`.
"""

import re
import shutil
import sys
from pathlib import Path
from xml.sax.saxutils import escape

SITE = "https://reflectapi.rs/"
SUMMARY_LINE = re.compile(r"^\s*- \[(?P<title>[^\]]+)\]\((?P<path>[^)]+\.md)\)")
SECTION_LINE = re.compile(r"^# (?P<title>.+)$")

docs = Path(__file__).resolve().parent
src = docs / "src"
book = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else docs / "book"
landing = docs / "landing"


def page_url(md_path: str) -> str:
    """The published URL of a source page, as mdBook names it."""
    path = md_path.removeprefix("./")
    name = path.rsplit("/", 1)[-1]
    if name in ("README.md", "index.md"):
        path = path.removesuffix(name)
    else:
        path = path.removesuffix(".md") + ".html"
    return SITE + path


def chapters():
    """(section, title, source path) for each page, in SUMMARY.md order."""
    section = None
    for line in (src / "SUMMARY.md").read_text().splitlines():
        if heading := SECTION_LINE.match(line):
            if heading["title"] != "Summary":
                section = heading["title"]
        elif chapter := SUMMARY_LINE.match(line):
            yield section, chapter["title"], chapter["path"].removeprefix("./")


def svg_description(page: Path, include: str) -> str:
    """An included drawing, as text: its <desc>, for llms-full.txt."""
    svg = (page.parent / include).read_text()
    desc = re.search(r"<desc[^>]*>(.*?)</desc>", svg, re.S)
    return f"(Diagram: {desc[1].strip()})" if desc else ""


MARKER = re.compile(r"\b(MUST NOT|MUST|SHOULD NOT|SHOULD|MAY NOT|MAY)\b")
RULE_TAG = re.compile(r"^\s*`\[(api-[a-z0-9-]+)\]`$")


def check_standard() -> list[str]:
    """The form of the API design standard's rules, as its "How to read it"
    gives it: a rule is a bullet whose first paragraph has a key word in
    capitals and closes with the rule's id, each id used once; and a key
    word in capitals appears nowhere else, except as code."""
    path = src / "design" / "standard.md"
    errors, ids = [], {}
    paragraph, bullet, start, fenced = [], False, 0, False

    def close():
        nonlocal paragraph, bullet
        if paragraph:
            tag = RULE_TAG.match(paragraph[-1])
            text = re.sub(r"`[^`]*`", "", " ".join(paragraph))
            marked = MARKER.search(text)
            where = f"{path.name}:{start}"
            if tag and not bullet:
                errors.append(f"{where}: the id {tag[1]} doesn't close a bullet")
            elif tag and not marked:
                errors.append(f"{where}: the rule {tag[1]} has no key word in capitals")
            elif marked and not tag:
                kind = "a rule without an id" if bullet else "outside a rule"
                errors.append(f"{where}: {marked[1]} {kind}: {paragraph[0].strip()[:60]}")
            if tag:
                if tag[1] in ids:
                    errors.append(f"{where}: the id {tag[1]} is also at line {ids[tag[1]]}")
                ids[tag[1]] = start
        paragraph, bullet = [], False

    for number, line in enumerate(path.read_text().splitlines(), 1):
        if line.lstrip().startswith("```"):
            close()
            fenced = not fenced
        elif fenced:
            continue
        elif not line.strip() or line.lstrip().startswith(("<", "|", "#")):
            close()
        elif re.match(r"\s*(-|\d+\.) ", line):
            close()
            paragraph, bullet, start = [line], True, number
        else:
            if not paragraph:
                start = number
            paragraph.append(line)
    close()
    if not ids:
        errors.append(f"{path.name}: no rules found")
    return errors


def main() -> int:
    if not book.is_dir():
        print(f"{book} does not exist: run `mdbook build` first", file=sys.stderr)
        return 1

    if errors := check_standard():
        print("\n".join(errors), file=sys.stderr)
        return 1

    for file in landing.iterdir():
        shutil.copy(file, book / file.name)
    # The landing page previews the standard's annotated example, inlined
    # from the book's copy so both stay one drawing.
    index_html = book / "index.html"
    index_html.write_text(
        re.sub(r"<!-- glance:[^>]*-->", lambda _: (src / "design" / "glance.svg").read_text().strip(),
               index_html.read_text())
    )

    pages = list(chapters())
    description = re.search(r'^description = "(.*)"$', (docs / "book.toml").read_text(), re.M)

    urls = [SITE] + [page_url(path) for _, _, path in pages]
    (book / "sitemap.xml").write_text(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        + "".join(f"  <url><loc>{escape(url)}</loc></url>\n" for url in dict.fromkeys(urls))
        + "</urlset>\n"
    )

    index = [
        "# reflectapi",
        "",
        f"> {description[1] if description else ''}",
        "",
        f"The whole guide as one file: {SITE}llms-full.txt",
        "API reference: https://docs.rs/reflectapi",
        "Source: https://github.com/thepartly/reflectapi",
    ]
    current = object()
    for section, title, path in pages:
        if section != current:
            current = section
            index += ["", f"## {section or 'Overview'}", ""]
        index.append(f"- [{title}]({page_url(path)})")
    (book / "llms.txt").write_text("\n".join(index) + "\n")

    full = [f"# reflectapi user guide\n\nSource: {SITE}\n"]
    for _, _, path in pages:
        text = re.sub(r"\{\{#include (\S+\.svg)\}\}", lambda m: svg_description(src / path, m[1]),
                      (src / path).read_text().strip())
        full.append(f"\n<!-- {page_url(path)} -->\n\n{text}\n")
    (book / "llms-full.txt").write_text("".join(full))

    print(f"wrote the landing page, sitemap.xml ({len(dict.fromkeys(urls))} URLs), llms.txt, llms-full.txt")
    return 0


if __name__ == "__main__":
    sys.exit(main())
