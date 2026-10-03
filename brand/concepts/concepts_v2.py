import math
import pathlib

from shapely import affinity
from shapely.geometry import Polygon

from geo import arc, box, disc, fit, path_d, rrect, stroke, union

OUT = pathlib.Path(__file__).resolve().parent / "v2"
OUT.mkdir(exist_ok=True)
CANVAS = 256


def svg(geom, name, field=200):
    d = path_d(fit(geom, field, CANVAS / 2))
    doc = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CANVAS} {CANVAS}">'
           f'<path d="{d}" fill="#161618" fill-rule="evenodd"/></svg>')
    (OUT / name).write_text(doc, encoding="utf-8")
    return OUT / name


def teardrop(cx, cy, R, H):
    center = (cx, cy)
    d = H
    theta = math.acos(min(1, R / d))
    t1 = (cx + R * math.sin(theta), cy - R * math.cos(theta))
    t2 = (cx - R * math.sin(theta), cy - R * math.cos(theta))
    cone = Polygon([(cx, cy - H), t1, t2])
    return union(disc(cx, cy, R), cone)


def concept_a():
    # Росчерк-комета: ядро уходит вверх-вправо, хвост тает в точки
    hx, hy = 162, 104
    head = affinity.rotate(teardrop(hx, hy, 42, 86), 45, origin=(hx, hy))
    tail = []
    for i in range(7):
        t = (i + 1) / 7
        tail.append(disc(hx - 96 * t, hy + 96 * t, 24 * (1 - t) ** 1.2 + 2))
    return union(head, *tail)


def concept_b():
    # Убывающие точки по разомкнутой дуге — присутствие тает
    cx, cy, R = 128, 128, 86
    discs = []
    n = 11
    a0, a1 = 150, -120
    for i in range(n):
        t = i / (n - 1)
        a = math.radians(a0 + (a1 - a0) * t)
        r = 26 * (1 - t) ** 1.35 + 3 * (1 - t) + 2.2
        discs.append(disc(cx + R * math.cos(a), cy + R * math.sin(a), r))
    return union(*discs)


def concept_c():
    # Карточка сайта рассыпается по сетке вправо
    card = rrect(16, 40, 150, 216, 26)
    bar = box(16, 40, 150, 78)
    dots = union(*(disc(40 + i * 20, 59, 5.5) for i in range(3)))
    body = card.difference(bar.buffer(0)).union(card.intersection(bar).difference(dots))
    left = body.intersection(box(0, 0, 150, 256))
    cells = []
    sizes = (26, 21, 16, 11)
    keep = {0: range(5), 1: (0, 1, 3, 4), 2: (0, 2, 4), 3: (1, 3)}
    for gx, s in enumerate(sizes):
        for gy in keep[gx]:
            cy = 60 + gy * 36
            x = 150 + gx * 26 + gx * gx * 6
            cells.append(box(x, cy - s / 2, x + s, cy + s / 2))
    return union(left, *cells)


if __name__ == "__main__":
    svg(concept_a(), "concept-a.svg")
    svg(concept_b(), "concept-b.svg")
    svg(concept_c(), "concept-c.svg")
    print("ok", OUT)
