#!/usr/bin/env python3
"""Finish the built book: the landing page, and the files search engines and
coding agents look for.

Run after `mdbook build`, from anywhere: `python3 docs/build-extras.py`, or
`python3 docs/build-extras.py DIR` for a book built with `mdbook build -d DIR`.

- `index.html`: the landing page from `docs/landing/`, in place of mdBook's
  copy of the first chapter, which stays at `introduction.html`.
- `sitemap.xml`: every page, for search engines.
- `llms.txt`: the guide's table of contents with links (https://llmstxt.org).
- `llms-full.txt`: the whole guide as one Markdown file, in reading order,
  so an agent can read all of it with one fetch.
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


def main() -> int:
    if not book.is_dir():
        print(f"{book} does not exist: run `mdbook build` first", file=sys.stderr)
        return 1

    for file in landing.iterdir():
        shutil.copy(file, book / file.name)

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
        full.append(f"\n<!-- {page_url(path)} -->\n\n{(src / path).read_text().strip()}\n")
    (book / "llms-full.txt").write_text("".join(full))

    print(f"wrote the landing page, sitemap.xml ({len(dict.fromkeys(urls))} URLs), llms.txt, llms-full.txt")
    return 0


if __name__ == "__main__":
    sys.exit(main())
