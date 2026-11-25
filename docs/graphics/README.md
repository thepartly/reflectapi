# Graphics

Sources for reflectapi's logo files and drawings:

- `build.py` draws `docs/src/reflectapi-wordmark.svg` and
  `reflectapi-wordmark-dark.svg` from the crab, `docs/src/reflectapi-logo.png`,
  and the Oxanium font, then renders `social-preview.html` to
  `social-preview.png`.
- `glance.py` draws `docs/src/design/glance.svg`, the standard's annotated
  example, which the book's standard and the landing page's preview share.
- `social-preview.png` is the repository's social preview image, uploaded by
  hand in the GitHub repository's Settings, under Social preview.

```bash
uv run docs/graphics/build.py   # set $CHROME if Chrome isn't on PATH
python3 docs/graphics/glance.py
```

The spacing and sizes are constants at the top of `build.py` and `glance.py`.
