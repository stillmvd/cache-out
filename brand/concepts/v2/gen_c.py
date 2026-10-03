import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from geo import box, fit, path_d, rrect, union

OUT = pathlib.Path(__file__).resolve().parent / "c"
OUT.mkdir(exist_ok=True)
INK = "#161618"
INKSCAPE = "C:/Program Files/Inkscape/bin/inkscape.com"
L = 170.0
C = L / 5.0            # клетка = 1/5 стороны
TOP = 128 - L / 2


def cell_box(gx, gy, s):
    cx = gx * C + C / 2
    cy = TOP + gy * C + C / 2
    return box(cx - s / 2, cy - s / 2, cx + s / 2, cy + s / 2)


def detach_col(style, gy):
    # с какого столбца клетки начинают откалываться (рваный край)
    if style == "even":
        return 3
    if style == "diagonal":
        return 2 + gy            # сверху откалывается раньше → диагональ
    return 4 - min(gy, 4)        # fade: сверху цел, низ сыпется — зеркальная диагональ


def build(style, name):
    body = rrect(0, TOP, L, L, L * 0.2)
    notches, flakes = [], []
    for gy in range(5):
        f = detach_col(style, gy)
        for gx in range(f, f + 5):
            step = gx - f                       # 0 = первый отколотый у грани
            s = C * (0.9 - step * 0.13)
            if s < 2.4:
                continue
            if step >= 2 and (gx + gy) % 2:      # прорежаем дальние
                continue
            home = cell_box(gx, gy, C * 0.96)
            if gx < 5:                            # дырка в теле только там, где было тело
                notches.append(home)
            gap = C * (0.16 + 0.55 * step + 0.14 * step * step)
            flakes.append(_shift(cell_box(gx, gy, s), gap))
    glyph = union(body.difference(union(*notches)), *flakes)
    doc = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
           f'<path d="{path_d(fit(glyph, 210, 128))}" fill="{INK}" fill-rule="evenodd"/></svg>')
    (OUT / name).write_text(doc, encoding="utf-8")
    return OUT / name


def _shift(geom, dx):
    from shapely import affinity
    return affinity.translate(geom, dx, 0)


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
                    "--export-dpi=96"], check=True, capture_output=True)
    return p.with_suffix(".png")


if __name__ == "__main__":
    files = [build("even", "c1-even.svg"), build("diagonal", "c2-diagonal.svg"), build("fade", "c3-fade.svg")]
    print(sheet(files))
