import math

from shapely import affinity
from shapely.geometry import Point, Polygon

from geo import INK, arc, box, disc, glyph, rrect, stroke, union


def crumb_trail(ox, oy, n, deg, length, r0, r1, spread=0.0):
    out = []
    dx, dy = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    px, py = -dy, dx
    for i in range(n):
        t = (i + 1) / n
        off = math.sin(t * math.pi) * spread
        out.append(disc(ox + dx * length * t + px * off,
                        oy + dy * length * t + py * off, r0 + (r1 - r0) * t))
    return union(*out)


def cookie(bite_deg=-38, chips=((34, 40, 6.5), (46, 66, 6), (64, 54, 5)), scallop=3):
    body = disc(50, 50, 38)
    cx, cy = 50 + 40 * math.cos(math.radians(bite_deg)), 50 + 40 * math.sin(math.radians(bite_deg))
    bites = [disc(cx, cy, 17)]
    for k in range(scallop):
        a = bite_deg - 26 + 52 * k / (scallop - 1)
        bites.append(disc(50 + 42 * math.cos(math.radians(a)), 50 + 42 * math.sin(math.radians(a)), 10))
    holes = union(*(disc(x, y, r) for x, y, r in chips))
    return body.difference(union(*bites)).difference(holes), (cx, cy)


def a1_classic():
    c, _ = cookie()
    crumbs = union(disc(90, 24, 4.5), disc(99, 40, 3.2))
    return union(c, crumbs)


def a2_burst():
    c, (bx, by) = cookie(bite_deg=-42)
    trail = crumb_trail(bx + 4, by - 4, 4, -40, 44, 5.2, 2.0, spread=4)
    return union(c, trail)


def a3_arrow():
    c, (bx, by) = cookie(bite_deg=-40, chips=((34, 42, 6.5), (44, 68, 6), (62, 58, 5)))
    shaft = stroke([(bx + 2, by - 1), (100, by - 22)], 5.5)
    ang = math.atan2(-22 - (by - 1 - by), 100 - (bx + 2))
    hx, hy = 100, by - 22
    head = Polygon([(hx + 11 * math.cos(ang), hy + 11 * math.sin(ang)),
                    (hx + 9 * math.cos(ang + 2.4), hy + 9 * math.sin(ang + 2.4)),
                    (hx + 9 * math.cos(ang - 2.4), hy + 9 * math.sin(ang - 2.4))])
    return union(c, shaft, head)


def window(x0, y0, x1, y1, bar=17, w=7, r=13):
    outer = rrect(x0, y0, x1, y1, r)
    inner = rrect(x0 + w, y0 + bar, x1 - w, y1 - w, max(4, r - 7))
    dots = union(*(disc(x0 + w + 7 + i * 11, y0 + (bar + w) / 2, 2.7) for i in range(3)))
    return outer.difference(inner).difference(dots)


def b1_motion():
    win = window(34, 18, 98, 80)
    lines = union(stroke([(4, 36), (24, 36)], 6), stroke([(-4, 50), (24, 50)], 6),
                  stroke([(8, 64), (24, 64)], 6))
    return union(win, lines)


def b2_arrow():
    win = window(8, 16, 70, 82)
    shaft = stroke([(44, 49), (100, 49)], 7)
    head = Polygon([(104, 49), (90, 40), (90, 58)])
    return union(win, shaft, head)


def b2_eject():
    win = window(6, 18, 66, 80)
    trail = union(crumb_trail(74, 36, 3, -8, 26, 5, 2.4),
                  crumb_trail(72, 52, 3, 4, 30, 5.5, 2.6),
                  crumb_trail(74, 66, 2, 16, 22, 4.5, 2.4))
    return union(win, trail)


VARIANTS = {
    "a1": a1_classic,
    "a2": a2_burst,
    "a3": a3_arrow,
    "b1": b1_motion,
    "b2": b2_arrow,
    "b3": b2_eject,
}
SHAPES = {}


def variant_svg(cid, key, in_circle):
    if key not in SHAPES:
        SHAPES[key] = VARIANTS[key]()
    return glyph(SHAPES[key], 58, INK)


NUMBER = 2
TITLE = "Волна 2 · доводка печенья и окна"
PROMPT = ("Метёлки в топку. Два фаворита, по три исполнения, чище линиями. Отметь, какая форма печенья и "
          "какое окно беру в доводку — можно по одному из каждого ряда или только один ряд.")
CHOSEN = None

CARDS = [
    ("A · надкушенное печенье", [
        {"code": "A1", "name": "классика", "note": "Чистый укус-полукруг справа, три капли, две крошки рядом. Спокойный, дружелюбный.",
         "tiles": [("A1", "", "a1", False)]},
        {"code": "A2", "name": "крошки наружу", "note": "Из укуса веером вылетают крошки вверх-вправо — движение «на выход».",
         "tiles": [("A2", "", "a2", False)]},
        {"code": "A3", "name": "укус + стрелка", "note": "Крошки собраны в стрелку наружу — прямее всего читает Cache Out, но деталей больше.",
         "tiles": [("A3", "", "a3", False)]},
    ]),
    ("B · окно браузера", [
        {"code": "B1", "name": "линии скорости", "note": "Окно с тремя кнопками, слева линии скорости — уезжает вправо.",
         "tiles": [("B1", "", "b1", False)]},
        {"code": "B2", "name": "стрелка наружу", "note": "Стрелка пробивает правую грань окна наружу. Буквальный «out».",
         "tiles": [("B2", "", "b2", False)]},
        {"code": "B3", "name": "выброс частиц", "note": "Из окна вправо вылетают частицы-данные — родня печенью-крошкам.",
         "tiles": [("B3", "", "b3", False)]},
    ]),
]

SUMMARY = ("Моё мнение: A2 — лучший баланс, печенье читается мгновенно и крошки дают смысл «out» без лишних "
           "деталей; A3 честнее по названию, но на 24 px стрелка липнет к телу. Из окон крепче B2 — стрелка "
           "наружу однозначна и держит силуэт; B1 на мелком размере теряет линии, B3 красив, но частицы на "
           "24 px сливаются в кашу. Сильная пара для финала: A2 и B2.")
