# Graphics

Sources for reflectapi's logo files:

- `build.py` draws `docs/src/reflectapi-wordmark.svg` and
  `reflectapi-wordmark-dark.svg` from the crab, `docs/src/reflectapi-logo.png`,
  and the Oxanium font, then renders `social-preview.html` to
  `social-preview.png`.
- `social-preview.png` is the repository's social preview image, uploaded by
  hand in the GitHub repository's Settings, under Social preview.

```bash
uv run docs/graphics/build.py   # set $CHROME if Chrome isn't on PATH
```

The spacing and sizes are constants at the top of `build.py`.
