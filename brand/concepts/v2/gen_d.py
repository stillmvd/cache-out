import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from geo import box, fit, path_d, rrect, union

OUT = pathlib.Path(__file__).resolve().parent / "d"
OUT.mkdir(exist_ok=True)
INK = "#161618"
INKSCAPE = "C:/Program Files/Inkscape/bin/inkscape.com"


def sq(cx, cy, s):
    return box(cx - s / 2, cy - s / 2, cx + s / 2, cy + s / 2)


def parts(n, radius, gone, flakes, size=212):
    from shapely import affinity
    holes = union(*[box(gx, gy, gx + 1, gy + 1) for gx, gy in gone])
    body = rrect(0, 0, n, n, n * radius).difference(holes)
    bits = union(*[sq(cx, cy, s) for cx, cy, s in flakes])
    x0, y0, x1, y1 = union(body, bits).bounds
    k = size / max(x1 - x0, y1 - y0)
    dx, dy = 128 - (x0 + x1) / 2 * k, 128 - (y0 + y1) / 2 * k
    place = lambda g: affinity.translate(affinity.scale(g, k, k, origin=(0, 0)), dx, dy)
    return place(body), place(bits)


def build(name, n, radius, gone, flakes):
    body = rrect(0, 0, n, n, n * radius)
    holes = union(*[box(gx, gy, gx + 1, gy + 1) for gx, gy in gone])
    bits = [sq(cx, cy, s) for cx, cy, s in flakes]
    glyph = union(body.difference(holes), *bits)
    doc = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
           f'<path d="{path_d(fit(glyph, 212, 128))}" fill="{INK}" fill-rule="evenodd"/></svg>')
    (OUT / name).write_text(doc, encoding="utf-8")
    return OUT / name


VARIANTS = [
    ("d1-corner.svg", 4, 0.22,
     [(2, 0), (3, 0), (3, 1)],
     [(3.1, -0.1, 0.78), (4.1, 0.9, 0.78), (4.7, -0.7, 0.58), (5.45, -1.45, 0.38)]),
    ("d2-edge.svg", 4, 0.22,
     [(3, 0), (3, 1), (2, 0), (3, 3)],
     [(3.95, 0.5, 0.78), (3.4, -0.45, 0.6), (4.75, 1.45, 0.72), (5.0, 3.45, 0.6), (5.85, 0.6, 0.4)]),
    ("d3-bold.svg", 3, 0.26,
     [(1, 0), (2, 0), (2, 1)],
     [(2.05, -0.05, 0.8), (3.05, 0.95, 0.8), (3.65, -0.65, 0.55)]),
]


def sheet(files):
    cols = []
    for i, f in enumerate(files):
        s = f.read_text(encoding="utf-8")
        inner = s[s.index(">") + 1:s.rindex("</svg>")]
        x = 16 + i * 230
        cols.append(f'<svg x="{x}" y="16" width="150" height="150" viewBox="0 0 256 256">{inner}</svg>')
        sx = x
        for px in (64, 48, 32, 16):
            cols.append(f'<svg x="{sx}" y="{180 + (64 - px)}" width="{px}" height="{px}" viewBox="0 0 256 256">{inner}</svg>')
            sx += px + 12
    w = 16 + len(files) * 230
    doc = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="266" viewBox="0 0 {w} 266">'
           f'<rect width="100%" height="100%" fill="#fff"/>{"".join(cols)}</svg>')
    p = OUT / "sheet.svg"
    p.write_text(doc, encoding="utf-8")
    subprocess.run([INKSCAPE, str(p), "--export-type=png", f"--export-filename={p.with_suffix('.png')}",
                    "--export-dpi=192"], check=True, capture_output=True)
    return p.with_suffix(".png")


if __name__ == "__main__":
    print(sheet([build(*v) for v in VARIANTS]))
