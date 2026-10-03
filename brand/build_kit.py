import pathlib
import subprocess
import sys

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "concepts"))
sys.path.insert(0, str(ROOT / "concepts" / "v2"))
from gen_d import VARIANTS, parts
from geo import box, path_d, rrect, union

OUT = ROOT / "logo"
PNG = OUT / "png"
ICONS = OUT / "icons"
INKSCAPE = "C:/Program Files/Inkscape/bin/inkscape.com"
INDIGO, CORAL, INK, PAPER = "#5B4BE8", "#FF6B5A", "#161618", "#ECECEF"
SPEC = {v[0]: v[1:] for v in VARIANTS}
ICON, SIMPLE = SPEC["d1-corner.svg"], SPEC["d3-bold.svg"]


def doc(w, h, body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">{body}</svg>'


def paint(body, bits, colors):
    if len(colors) == 1:
        return f'<path d="{path_d(body)}{path_d(bits)}" fill="{colors[0]}" fill-rule="evenodd"/>'
    return f'<path d="{path_d(body)}" fill="{colors[0]}"/><path d="{path_d(bits)}" fill="{colors[1]}"/>'


def mark(spec, colors, size=224):
    return doc(256, 256, paint(*parts(*spec, size=size), colors))


def pixel(spec, px, colors):
    n, radius, gone, flakes = spec
    top = min(cy - s / 2 for _, cy, s in flakes)
    right = max(cx + s / 2 for cx, _, s in flakes)
    u = int(px // max(right, n - top))
    ox = (px - round(right * u)) // 2
    oy = (px - round((n - top) * u)) // 2 - round(top * u)
    holes = union(*[box(gx * u, gy * u, (gx + 1) * u, (gy + 1) * u) for gx, gy in gone])
    body = rrect(0, 0, n * u, n * u, round(n * radius * u)).difference(holes)
    cubes = []
    for cx, cy, s in flakes:
        side = round(s * u)
        x0, y0 = round(cx * u - side / 2), round(cy * u - side / 2)
        cubes.append(box(x0, y0, x0 + side, y0 + side))
    from shapely import affinity
    shift = lambda g: affinity.translate(g, ox, oy)
    return doc(px, px, paint(shift(body), shift(union(*cubes)), colors))


def wordmark(text, height_cap):
    font = TTFont(ROOT / "fonts" / "KockersSans-Bold.woff2")
    gs, cmap = font.getGlyphSet(), font.getBestCmap()
    k = height_cap / font["OS/2"].sCapHeight
    pen, x = SVGPathPen(gs), 0
    for ch in text:
        name = cmap[ord(ch)]
        gs[name].draw(TransformPen(pen, (k, 0, 0, -k, x, 0)))
        x += font["hmtx"][name][0] * k
    return pen.getCommands(), x


def lockup(text_color, mark_colors):
    body, bits = parts(*ICON, size=256)
    bx0, by0, bx1, by1 = body.bounds
    cap = (by1 - by0) * 0.5
    d, width = wordmark("Cache Out", cap)
    tx, base = bx1 + (bx1 - bx0) * 0.21, (by0 + by1) / 2 + cap / 2
    w = round(tx + width + 4)
    return doc(w, 256, paint(body, bits, mark_colors)
               + f'<path transform="translate({tx:.2f} {base:.2f})" d="{d}" fill="{text_color}"/>')


def render(svg, png, w, h=None):
    subprocess.run([INKSCAPE, str(svg), "--export-type=png", f"--export-filename={png}",
                    f"--export-width={w}", f"--export-height={h or w}"], check=True, capture_output=True)
    return png


def write(name, text):
    p = OUT / name
    p.write_text(text, encoding="utf-8")
    return p


def main():
    for d in (OUT, PNG, ICONS):
        d.mkdir(exist_ok=True)
    masters = {
        "cacheout-icon.svg": mark(ICON, (INDIGO, CORAL)),
        "cacheout-icon-indigo.svg": mark(ICON, (INDIGO,)),
        "cacheout-icon-black.svg": mark(ICON, ("#000000",)),
        "cacheout-icon-white.svg": mark(ICON, ("#FFFFFF",)),
        "cacheout-simple.svg": mark(SIMPLE, (INDIGO, CORAL)),
        "cacheout-simple-indigo.svg": mark(SIMPLE, (INDIGO,)),
        "cacheout-simple-black.svg": mark(SIMPLE, ("#000000",)),
        "cacheout-simple-white.svg": mark(SIMPLE, ("#FFFFFF",)),
        "cacheout-lockup.svg": lockup(INK, (INDIGO, CORAL)),
        "cacheout-lockup-dark.svg": lockup(PAPER, (INDIGO, CORAL)),
        "cacheout-lockup-black.svg": lockup("#000000", ("#000000",)),
        "cacheout-lockup-white.svg": lockup("#FFFFFF", ("#FFFFFF",)),
    }
    files = {n: write(n, t) for n, t in masters.items()}
    for px in (16, 20, 24, 32):
        files[f"px{px}"] = write(f"cacheout-simple-{px}px.svg", pixel(SIMPLE, px, (INDIGO, CORAL)))
    for px in (40, 48, 64):
        files[f"px{px}"] = write(f"cacheout-icon-{px}px.svg", pixel(ICON, px, (INDIGO, CORAL)))

    for n in ("cacheout-icon.svg", "cacheout-simple.svg"):
        for px in (256, 512, 1024):
            render(files[n], PNG / f"{n[:-4]}-{px}.png", px)
    for n in ("cacheout-lockup.svg", "cacheout-lockup-dark.svg"):
        w = int(masters[n].split('width="')[1].split('"')[0])
        render(files[n], PNG / f"{n[:-4]}-1200.png", 1200, round(1200 * 256 / w))

    frames = {}
    for px in (16, 20, 24, 32, 40, 48, 64):
        frames[px] = render(files[f"px{px}"], ICONS / f"_{px}.png", px)
    for px in (128, 256):
        frames[px] = render(files["cacheout-icon.svg"], ICONS / f"_{px}.png", px)
    imgs = {px: Image.open(p).convert("RGBA") for px, p in frames.items()}
    big = imgs[256]
    big.save(ICONS / "icon.ico", format="ICO", sizes=[(p, p) for p in imgs],
             append_images=[imgs[p] for p in imgs if p != 256])
    for name, px in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256)):
        imgs[px].save(ICONS / name)
    render(files["cacheout-icon.svg"], ICONS / "icon.png", 1024)
    for px, p in frames.items():
        p.unlink()
    print("ok", len(list(OUT.rglob("*.*"))), "files")


if __name__ == "__main__":
    main()
