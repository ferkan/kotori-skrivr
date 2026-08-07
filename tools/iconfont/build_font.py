import pathops
from fontTools.svgLib.path import parse_path
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.fontBuilder import FontBuilder
from fontTools.misc.transform import Transform

EM, BOX = 1000, 860
ICONS = ['bold','italic','code-block','quote','link','unlink',
 'list-bullet-unordered','list-bullet-numbers','list-bullet-letters',
 'outline','anfang','erase','save','print','copy','cut','paste']

def load(name):
    d = open(f'work/raw/{name}.path').read()
    p = pathops.Path()
    parse_path(d, p.getPen())
    # The outliner emits even-odd contours; simplify re-winds them for the
    # nonzero fill that TrueType glyphs require, so interiors stay hollow.
    p.fillType = pathops.FillType.EVEN_ODD
    return pathops.simplify(p, fix_winding=True, clockwise=False)

glyphs, widths, report = {}, {}, []
for i, name in enumerate(ICONS):
    p = load(name)
    x0, y0, x1, y1 = p.bounds
    w, h = x1 - x0, y1 - y0
    s = BOX / max(w, h)
    # y-flip: SVG is y-down, font coords are y-up.
    tx = (EM - w * s) / 2 - x0 * s
    ty = (EM - h * s) / 2 + y1 * s
    pen = TTGlyphPen(None)
    p.draw(TransformPen(Cu2QuPen(pen, max_err=1.0), Transform(s, 0, 0, -s, tx, ty)))
    g = name.replace('-', '_')
    glyphs[g] = pen.glyph()
    widths[g] = EM
    report.append((name, round(w*s), round(h*s)))

order = ['.notdef'] + [n.replace('-', '_') for n in ICONS]
glyphs['.notdef'] = TTGlyphPen(None).glyph()
widths['.notdef'] = EM
cmap = {0xE001 + i: n.replace('-', '_') for i, n in enumerate(ICONS)}

fb = FontBuilder(EM, isTTF=True)
fb.setupGlyphOrder(order)
fb.setupCharacterMap(cmap)
fb.setupGlyf(glyphs)
fb.setupHorizontalMetrics({g: (widths[g], 0) for g in order})
fb.setupHorizontalHeader(ascent=880, descent=-120)
fb.setupNameTable({"familyName": "Skrivr Icons", "styleName": "Regular",
                   "psName": "SkrivrIcons-Regular", "version": "1.0"})
fb.setupOS2(sTypoAscender=880, sTypoDescender=-120, usWinAscent=880, usWinDescent=120)
fb.setupPost()
fb.save('work/SkrivrIcons.ttf')

print(f"built {len(ICONS)} glyphs")
for n, w, h in report:
    print(f"  {n:<24} {w:>4}x{h:<4}")
