import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent
PAGE = ROOT / "concepts-page.html"
INK, PAPER = "#161618", "#ececef"

CARDS = [
    ("d1-corner.svg", "D1 · Угол", "Плитка 4×4, правый верхний угол выкрошен ступенькой, кубики уходят по диагонали и мельчают.", True),
    ("d2-edge.svg", "D2 · Рваный край", "Правая грань рваная на разную глубину, кубики разлетаются вправо веером.", False),
    ("d3-bold.svg", "D3 · Крупно", "Та же идея на сетке 3×3: меньше деталей, самый сильный на 16 px.", False),
]


def inner(name, color):
    s = (ROOT / "d" / name).read_text(encoding="utf-8")
    return s[s.index(">") + 1:s.rindex("</svg>")].replace(INK, color)


def svg(name, color, px):
    return f'<svg viewBox="0 0 256 256" width="{px}" height="{px}">{inner(name, color)}</svg>'


def card(name, title, note, rec):
    sizes = "".join(
        f'<div class="sz"><div class="szbox {bg}">{svg(name, col, px)}</div><span>{px}</span></div>'
        for bg, col in (("light", INK), ("dark", PAPER)) for px in (48, 32, 16))
    badge = '<span class="rec">рекомендую</span>' if rec else ""
    return (f'<article class="card{" recc" if rec else ""}"><div class="big light">{svg(name, INK, 150)}</div>'
            f'<div class="big dark">{svg(name, PAPER, 150)}</div><div class="sizes">{sizes}</div>'
            f'<div class="meta"><h2>{title}{badge}</h2><p>{note}</p></div></article>')


def main():
    html = PAGE.read_text(encoding="utf-8")
    if "<!--wave-d-->" in html:
        html = re.sub(r"<!--wave-d-->[\s\S]*?<!--/wave-d-->", "", html)
    else:
        html = html.replace('<p class="lead">', '<!--old--><h2 class="wave">Волна C · отклонена: грань целая, у C1 «расчёска»</h2><p class="lead">', 1)
    head = ('<!--wave-d--><p class="lead">Волна D: скол по сетке. Выкрошенная клетка тела = кубик, который от неё отлетает; '
            'все летят в одну сторону и мельчают с расстоянием. Дырок внутри нет, грань в месте скола ступенчатая. Выбор напиши в чат.</p>'
            f'<div class="grid">{"".join(card(*c) for c in CARDS)}</div><div style="height:40px"></div><!--/wave-d-->')
    html = html.replace("<!--old-->", head + "<!--old-->", 1)
    html = html.replace(".lead{", "h2.wave{font-size:18px;margin:0 0 6px;color:var(--dim)}.lead{", 1) if "h2.wave{" not in html else html
    PAGE.write_text(html, encoding="utf-8")
    print("ok", len(html))


if __name__ == "__main__":
    main()
