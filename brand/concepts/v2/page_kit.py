import base64
import io
import pathlib
import re

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent
PAGE = ROOT / "concepts-page.html"
LOGO = ROOT.parent.parent / "logo"


def svg(name, height):
    s = (LOGO / name).read_text(encoding="utf-8")
    return re.sub(r'width="\d+" height="\d+"', f'height="{height}" style="max-width:100%;height:auto;max-height:{height}px"', s, count=1)


def frames():
    ico = Image.open(LOGO / "icons" / "icon.ico")
    out = []
    for s in sorted(ico.info["sizes"]):
        if s[0] > 64:
            continue
        ico.size = s
        buf = io.BytesIO()
        ico.copy().save(buf, "PNG")
        uri = base64.b64encode(buf.getvalue()).decode()
        out.append(f'<div class="sz"><img src="data:image/png;base64,{uri}" width="{s[0] * 2}" height="{s[0] * 2}" '
                   f'style="image-rendering:pixelated" alt=""><span>{s[0]}</span></div>')
    return "".join(out)


def taskbar(bg, fg):
    ico = svg("cacheout-simple-24px.svg", 24)
    dots = "".join(f'<i style="background:{fg}"></i>' for _ in range(4))
    return f'<div class="tb" style="background:{bg}">{dots}<b>{ico}</b>{dots[:len(dots) // 2]}</div>'


def main():
    html = PAGE.read_text(encoding="utf-8")
    html = re.sub(r"<!--kit-->[\s\S]*?<!--/kit-->", "", html)
    f = frames()
    swatches = "".join(f'<div class="sw"><div style="background:{h}"></div><b>{n}</b><code>{h}</code></div>'
                       for n, h in (("Индиго", "#5B4BE8"), ("Коралл", "#FF6B5A"), ("Ink", "#161618"), ("Paper", "#ECECEF")))
    block = ('<!--kit--><p class="lead">Итог: D1 — иконка приложения, D3 — простая версия, цвет P6. '
             'Файлы — <code>brand/logo/</code>, правила — <code>brand/BRAND.md</code>.</p>'
             '<div class="kit">'
             f'<div class="big light lk">{svg("cacheout-lockup.svg", 110)}</div>'
             f'<div class="big dark lk">{svg("cacheout-lockup-dark.svg", 110)}</div>'
             '<div class="row">'
             f'<div class="big light sq">{svg("cacheout-icon.svg", 130)}</div>'
             f'<div class="big light sq">{svg("cacheout-simple.svg", 130)}</div>'
             f'<div class="big light sq">{svg("cacheout-icon-indigo.svg", 130)}</div>'
             f'<div class="big dark sq">{svg("cacheout-icon-white.svg", 130)}</div></div>'
             f'<h2>icon.ico: 16–32 — D3, 40–64 — D1, по пикселям (×2)</h2>'
             f'<div class="frames light">{f}</div><div class="frames dark">{f}</div>'
             f'<h2>Панель задач, 24 px</h2>{taskbar("#f3f3f5", "#c9c9cf")}{taskbar("#1c1c1f", "#3a3a40")}'
             f'<div class="sws">{swatches}</div></div>'
             '<div style="height:40px"></div><h2 class="wave">Цвет · выбрано: P6 · Индиго + коралл</h2><!--/kit-->')
    html = html.replace("<!--color-->", block + "<!--color-->", 1)
    css = (".big.dark,.frames.dark{border:1px solid var(--line)}.kit{display:grid;gap:16px}.kit .row{display:grid;gap:16px;grid-template-columns:repeat(auto-fit,minmax(min(100%,180px),1fr))}"
           ".big.lk{height:auto;padding:28px 20px}.big.sq{height:170px}"
           ".frames{display:flex;flex-wrap:wrap;align-items:flex-end;gap:18px;padding:18px;border-radius:16px}"
           ".frames.light{background:#fff}.frames.dark{background:#141416}.frames .sz span{color:#8a8a92}"
           ".tb{display:flex;justify-content:center;align-items:center;gap:10px;height:48px;border-radius:12px}"
           ".tb i{width:24px;height:24px;border-radius:6px;display:block}.tb b{display:grid;place-items:center;width:40px;height:40px;"
           "border-radius:8px;background:rgba(128,128,140,.18)}"
           ".sws{display:flex;flex-wrap:wrap;gap:16px}.sw{display:grid;gap:4px;font-size:13px}"
           ".sw div{width:120px;height:64px;border-radius:12px;border:1px solid var(--line)}")
    if ".kit{" not in html:
        html = html.replace("</style>", css + "</style>", 1)
    PAGE.write_text(html, encoding="utf-8")
    print("ok", len(html))


if __name__ == "__main__":
    main()
