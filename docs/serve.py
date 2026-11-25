#!/usr/bin/env python3
"""Serve the whole site locally, as it is published: the book, the landing
page, and what `build-extras.py` adds. Rebuilds when a source changes, and
reloads the pages open in a browser after each rebuild.

    python3 docs/serve.py          # http://127.0.0.1:3000/
    python3 docs/serve.py 8080

`mdbook serve` serves the book alone, without the landing page. This needs
`mdbook` (`cargo install mdbook --locked`), and nothing else.
"""

import functools
import http.server
import os
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

docs = Path(__file__).resolve().parent
port = int(sys.argv[1]) if len(sys.argv) > 1 else 3000
watched = [docs / "src", docs / "theme", docs / "landing", docs / "book.toml", docs / "build-extras.py"]

# mdbook, from PATH, or from where `cargo install` puts it.
mdbook = shutil.which("mdbook") or shutil.which("mdbook", path=str(Path.home() / ".cargo" / "bin"))

builds = 0  # counts rebuilds: pages poll it, and reload when it changes
started = int(time.time())  # so a restarted server reloads open pages too

RELOAD = b"""<script>
(() => {
  let seen = null;
  setInterval(async () => {
    try {
      const now = await (await fetch("/__build", {cache: "no-store"})).text();
      if (seen !== null && now !== seen) location.reload();
      seen = now;
    } catch {}
  }, 700);
})();
</script>"""


def snapshot() -> dict:
    """The modification time of every watched file."""
    stamps = {}
    for root in watched:
        # Files can vanish mid-scan (a git checkout, an editor's save): skip them.
        try:
            files = [root] if root.is_file() else [p for p in root.rglob("*") if p.is_file()]
        except OSError:
            continue
        for file in files:
            try:
                stamps[file] = file.stat().st_mtime
            except OSError:
                pass
    return stamps


def build() -> None:
    global builds
    subprocess.run([mdbook, "build"], cwd=docs, check=False)
    subprocess.run([sys.executable, str(docs / "build-extras.py")], check=False)
    builds += 1
    print("rebuilt", time.strftime("%H:%M:%S"), flush=True)


def watch() -> None:
    last = snapshot()
    while True:
        time.sleep(1)
        try:
            if snapshot() != last:
                time.sleep(0.5)  # let a burst of changes settle
                last = snapshot()
                build()
        except Exception as error:  # keep watching whatever happens
            print("watch error:", error, flush=True)


class Handler(http.server.SimpleHTTPRequestHandler):
    """The built site, never cached, each page with the reload script."""

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, format, *args):
        if "/__build" not in str(args[0] if args else ""):
            super().log_message(format, *args)

    def do_GET(self):
        if self.path == "/__build":
            return self.reply(f"{started}-{builds}".encode(), "text/plain")
        path = self.translate_path(self.path)
        if os.path.isdir(path) and self.path.split("?")[0].endswith("/"):
            path = os.path.join(path, "index.html")
        if path.endswith(".html") and os.path.isfile(path):
            page = Path(path).read_bytes()
            at = page.rfind(b"</body>")
            page = page + RELOAD if at < 0 else page[:at] + RELOAD + page[at:]
            return self.reply(page, "text/html; charset=utf-8")
        super().do_GET()

    def reply(self, body: bytes, content_type: str):
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main() -> int:
    if not mdbook:
        print("mdbook is not installed: cargo install mdbook --locked", file=sys.stderr)
        return 1
    build()
    threading.Thread(target=watch, daemon=True).start()
    handler = functools.partial(Handler, directory=str(docs / "book"))
    print(f"serving http://127.0.0.1:{port}/", flush=True)
    try:
        http.server.ThreadingHTTPServer(("127.0.0.1", port), handler).serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
