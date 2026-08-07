"""Instance the four static Literata cuts the editor needs.

The codebase registers a separate static family per weight/style and has no
variable-axis support, so the variable source has to be pinned down to discrete
files. `opsz` is pinned to 16 — the editor's body size — which buys the 16pt
optical master's larger apertures and looser spacing over the 12pt default.
"""
from fontTools import configLogger
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
import os

configLogger(level="ERROR")
SRC = "literata-main/fonts/variable"
OUT = "out"
os.makedirs(OUT, exist_ok=True)

OPSZ = 16
CUTS = [
    ("Literata[opsz,wght].ttf",        400, "Regular",        "Literata-Regular.ttf"),
    ("Literata[opsz,wght].ttf",        600, "SemiBold",       "Literata-SemiBold.ttf"),
    ("Literata-Italic[opsz,wght].ttf", 400, "Italic",         "Literata-Italic.ttf"),
    ("Literata-Italic[opsz,wght].ttf", 600, "SemiBold Italic","Literata-SemiBoldItalic.ttf"),
]

for src, wght, style, dest in CUTS:
    font = TTFont(os.path.join(SRC, src))
    inst = instancer.instantiateVariableFont(font, {"opsz": OPSZ, "wght": wght}, inplace=False)
    # Name the instance for what it is, so the OS/egui report it sensibly.
    name = inst["name"]
    name.setName("Literata", 1, 3, 1, 0x409)
    name.setName(style, 2, 3, 1, 0x409)
    name.setName(f"Literata {style}", 4, 3, 1, 0x409)
    name.setName(f"Literata-{style.replace(' ', '')}", 6, 3, 1, 0x409)
    path = os.path.join(OUT, dest)
    inst.save(path)
    upem = inst["head"].unitsPerEm
    hhea = inst["hhea"]
    native = (hhea.ascent - hhea.descent + hhea.lineGap) / upem
    print(f"{dest:<30} {os.path.getsize(path)/1024:>7.0f} KB  glyphs={inst['maxp'].numGlyphs:<6} native leading={native:.3f}")
