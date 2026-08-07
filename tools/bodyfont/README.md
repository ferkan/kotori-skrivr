# Literata body font

Regenerates the four static Literata cuts the editor embeds.

The codebase registers a **separate static family per weight/style**
(`FONT_LITERATA`, `FONT_LITERATA_BOLD`, …) and has no variable-axis support, so
the variable source must be pinned down to discrete files.

`opsz` is pinned to **16** — the editor's default body size — which uses the
16pt optical master's larger apertures and looser spacing rather than the
variable font's 12pt default. `wght` is pinned to **400** and **600**;
600 rather than 700 because a 700-weight serif H1 at 28px shouts, and inline
`**bold**` at 700 breaks up the paragraph texture.

## Rebuild

```sh
curl -sSL -o lit.zip https://github.com/googlefonts/literata/archive/refs/heads/main.zip
unzip -q lit.zip 'literata-main/fonts/variable/*' 'literata-main/OFL.txt'
pip install fonttools
python3 build_body.py     # -> out/Literata-{Regular,Italic,SemiBold,SemiBoldItalic}.ttf
```

Copy the results to `assets/fonts/`.

Licence: SIL Open Font License 1.1 — see `assets/fonts/Literata-LICENSE.txt`.
