import pathlib
import re

from gen_d import VARIANTS, parts
from geo import path_d

ROOT = pathlib.Path(__file__).resolve().parent
PAGE = ROOT / "concepts-page.html"
GEOM = {v[0]: parts(*v[1:]) for v in VARIANTS}
ICON, SIMPLE = "d1-corner.svg", "d3-bold.svg"

PALETTES = [
    ("P1 · Мандарин", "моно", ("#FF6B2C",)),
    ("P2 · Фуксия", "моно", ("#EC3D86",)),
    ("P3 · Коралл", "моно", ("#FF5A52",)),
    ("P4 · Электрик-индиго", "моно", ("#6A5AF0",)),
    ("P5 · Глубокая бирюза", "моно", ("#0E9F94",)),
    ("P6 · Индиго + коралл", "тело и кубики разным цветом", ("#5B4BE8", "#FF6B5A")),
    ("P7 · Закат", "градиент вдоль полёта: фуксия → мандарин", ("#EC3D86", "#FF9F1C", "grad")),
    ("P8 · Глубина", "градиент вдоль полёта: индиго → бирюза", ("#5B4BE8", "#1FC2B0", "grad")),
]


def mark(name, colors, px, uid):
    body, bits = GEOM[name]
    b, f = path_d(body), path_d(bits)
    if len(colors) == 1:
        inner = f'<path d="{b}{f}" fill="{colors[0]}" fill-rule="evenodd"/>'
    elif len(colors) == 2:
        inner = f'<path d="{b}" fill="{colors[0]}"/><path d="{f}" fill="{colors[1]}"/>'
    else:
        gid = f"g{uid}"
        inner = (f'<defs><linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="40" y1="216" x2="216" y2="40">'
                 f'<stop offset=".25" stop-color="{colors[0]}"/><stop offset="1" stop-color="{colors[1]}"/></linearGradient></defs>'
                 f'<path d="{b}{f}" fill="url(#{gid})" fill-rule="evenodd"/>')
    return f'<svg viewBox="0 0 256 256" width="{px}" height="{px}">{inner}</svg>'


def card(i, title, note, colors):
    n = [0]

    def m(name, px):
        n[0] += 1
        return mark(name, colors, px, f"{i}_{n[0]}")

    sizes = "".join(
        f'<div class="sz"><div class="szbox {bg}">{m(nm, px)}</div><span>{lbl}</span></div>'
        for bg in ("light", "dark") for nm, px, lbl in ((ICON, 40, "D1·40"), (SIMPLE, 32, "D3·32"), (SIMPLE, 16, "D3·16")))
    hexes = " · ".join(c for c in colors if c.startswith("#"))
    return (f'<article class="card"><div class="duo"><div class="big light">{m(ICON, 120)}</div>'
            f'<div class="big dark">{m(ICON, 120)}</div></div><div class="sizes c">{sizes}</div>'
            f'<div class="meta"><h2>{title}</h2><p>{note} — <code>{hexes}</code></p></div></article>')


def main():
    html = PAGE.read_text(encoding="utf-8")
    html = re.sub(r"<!--color-->[\s\S]*?<!--/color-->", "", html)
    block = ('<!--color--><p class="lead">Цвет. Один и тот же цвет на светлой и тёмной теме: сверху D1 (иконка приложения), '
             'снизу D3 в 32 и 16 px. Чистые синий и зелёный — клише чистильщиков, поэтому индиго и бирюза сдвинуты от них. Выбор напиши в чат.</p>'
             f'<div class="grid">{"".join(card(i, *p) for i, p in enumerate(PALETTES))}</div>'
             '<div style="height:40px"></div><h2 class="wave">Волна D · выбрано: D1 — иконка, D3 — упрощённая версия</h2><!--/color-->')
    html = html.replace("<!--wave-d-->", block + "<!--wave-d-->", 1)
    css = ".duo{display:grid;grid-template-columns:1fr 1fr;gap:10px}.duo .big{height:150px}code{font-size:12px}.sizes.c{gap:6px}.sizes.c .szbox{width:48px;height:48px}"
    if ".duo{" not in html:
        html = html.replace("</style>", css + "</style>", 1)
    html = html.replace("<h1>Знак Cache Out — скол плитки", "<h1>Знак Cache Out", 1)
    PAGE.write_text(html, encoding="utf-8")
    print("ok", len(html))


if __name__ == "__main__":
    main()
