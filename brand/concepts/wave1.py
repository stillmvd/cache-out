import math

from shapely import affinity
from shapely.geometry import Point

from geo import INK, arc, box, disc, glyph, rrect, stroke, union


def cookie_bite():
    body = disc(50, 50, 40)
    bite = union(*(disc(50 + 46 * math.cos(math.radians(a)), 50 + 46 * math.sin(math.radians(a)), 14)
                   for a in (-75, -45, -15)))
    chips = union(disc(31, 42, 6), disc(44, 70, 6), disc(64, 60, 5.5))
    crumbs = union(disc(97, 12, 4.5), disc(106, 34, 3.5))
    return union(body.difference(bite).difference(chips), crumbs)


def cookie_pixels():
    body = disc(50, 50, 40)
    split = 54
    left = body.intersection(box(0, 0, split, 100)).difference(union(disc(30, 40, 6), disc(38, 68, 6)))
    cells = []
    sizes = (12, 10, 8, 6)
    skip = {(1, 1), (2, 0), (2, 3), (2, 5), (3, 0), (3, 2), (3, 4), (3, 6)}
    for gx, size in enumerate(sizes):
        for gy in range(7):
            cy = 10 + gy * 13.5
            if (gx, gy) in skip or not body.contains(Point(split + 6 + gx * 8, cy)):
                continue
            x = split + 6 + gx * 13 + gx * gx * 4
            cells.append(box(x, cy - size / 2 - gx * 3, x + size, cy + size / 2 - gx * 3))
    return union(left, *cells)


def window_frame(x0, y0, x1, y1, w=7):
    outer = rrect(x0, y0, x1, y1, 12)
    inner = rrect(x0 + w, y0 + w + 10, x1 - w, y1 - w, 6)
    return outer.difference(inner)


def window_swoosh():
    frame = window_frame(0, 10, 92, 86)
    path = arc(10, 112, 78, -88, -20)
    swoosh = stroke(path, 9)
    gap = stroke(path, 17)
    dust = union(disc(102, 52, 4.5), disc(110, 70, 3.5), disc(98, 84, 3))
    return union(frame.difference(gap), swoosh, dust)


def window_out():
    frame = window_frame(26, 10, 110, 82)
    lines = union(stroke([(2, 34), (16, 34)], 7), stroke([(-8, 50), (16, 50)], 7), stroke([(6, 66), (16, 66)], 7))
    return union(frame, lines)


def broom():
    handle = stroke([(96, 4), (52, 52)], 8)
    head = affinity.rotate(rrect(26, 46, 70, 60, 5), -42, origin=(48, 53))
    fan = affinity.rotate(
        union(*(stroke([(30 + i * 8, 62), (24 + i * 9.5, 98)], 6, "flat") for i in range(6))), -42, origin=(48, 53))
    dust = union(disc(8, 96, 4.5), disc(20, 108, 3.5), disc(4, 80, 3))
    return union(handle, head, fan, dust)


def dust_brush():
    grip = rrect(20, 10, 96, 34, 12)
    bristles = union(*(stroke([(28 + i * 11, 34), (24 + i * 12, 64)], 6.5, "flat") for i in range(7)))
    brush = affinity.rotate(union(grip, bristles), -18, origin=(58, 36))
    sweep = stroke(arc(52, 40, 52, 112, 160), 6)
    dust = union(disc(4, 86, 4.5), disc(14, 98, 3.5), disc(-4, 100, 3))
    return union(brush, sweep, dust)


VARIANTS = {
    "a1": cookie_bite,
    "a2": cookie_pixels,
    "b1": window_swoosh,
    "b2": window_out,
    "c1": broom,
    "c2": dust_brush,
}
SHAPES = {}


def variant_svg(cid, key, in_circle):
    if key not in SHAPES:
        SHAPES[key] = VARIANTS[key]()
    return glyph(SHAPES[key], 58, INK)


NUMBER = 1
TITLE = "Волна 1 · три направления"
PROMPT = ("Три метафоры по два варианта, чёрным без подложки — сначала силуэт. Отметь всё, что цепляет, "
          "даже если только идея, а не исполнение.")
CHOSEN = ["A1 · печенье-укус", "B2 · окно на выход"]

CARDS = [
    ("A · надкушенное печенье", [
        {"code": "A1", "name": "укус и крошки", "note": "Классика куки: три шоколадные капли, укус, две крошки вылетают.",
         "tiles": [("A1", "", "a1", False)]},
        {"code": "A2", "name": "печенье в пиксели", "note": "Половина печенья рассыпается в квадраты и улетает — «кеш на выход».",
         "tiles": [("A2", "", "a2", False)]},
    ]),
    ("B · смахнуть с окна", [
        {"code": "B1", "name": "мах по окну", "note": "Окно браузера, дугой смахиваем — пыль слетает за край.",
         "tiles": [("B1", "", "b1", False)]},
        {"code": "B2", "name": "окно на выход", "note": "Окно уезжает вправо, за ним линии скорости — буквально Cache Out.",
         "tiles": [("B2", "", "b2", False)]},
    ]),
    ("C · вымести пыль", [
        {"code": "C1", "name": "метла", "note": "Метла по диагонали, перед ней три пылинки.",
         "tiles": [("C1", "", "c1", False)]},
        {"code": "C2", "name": "сметка", "note": "Щётка-сметка с дугой взмаха, пыль отлетает влево.",
         "tiles": [("C2", "", "c2", False)]},
    ]),
]

SUMMARY = ("Моё мнение: сильнее всего A1 и A2 — печенье узнаётся за долю секунды даже на 24 px, и оно же "
           "говорит «куки», то есть главное, о чём утилита. A2 лучше ложится на название (кеш уходит), "
           "A1 — дружелюбнее. Метла и сметка понятны, но это стоковые знаки «уборки» без привязки к браузеру; "
           "окна на 24 px теряют детали и превращаются в квадрат.")
