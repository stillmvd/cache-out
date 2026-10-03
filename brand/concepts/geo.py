import math

from shapely import affinity
from shapely.geometry import LineString, Point, Polygon, box
from shapely.ops import unary_union

INK = "#161618"
PAPER = "#f4f4f6"


def disc(x, y, r):
    return Point(x, y).buffer(r, quad_segs=48)


def stroke(points, width, cap="round"):
    return LineString(points).buffer(width / 2, quad_segs=24, cap_style=cap, join_style="round")


def rrect(x0, y0, x1, y1, r):
    return box(x0 + r, y0 + r, x1 - r, y1 - r).buffer(r, quad_segs=24)


def arc(cx, cy, r, a0, a1, steps=64):
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / steps)),
             cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / steps))) for i in range(steps + 1)]


def union(*shapes):
    return unary_union(shapes)


def fit(geom, size, center=32.0):
    x0, y0, x1, y1 = geom.bounds
    k = size / max(x1 - x0, y1 - y0)
    g = affinity.scale(geom, k, k, origin=(0, 0))
    gx0, gy0, gx1, gy1 = g.bounds
    return affinity.translate(g, center - (gx0 + gx1) / 2, center - (gy0 + gy1) / 2)


def ring_d(coords):
    pts = list(coords)[:-1]
    return "M" + "L".join(f"{x:.2f} {y:.2f}" for x, y in pts) + "Z"


def path_d(geom):
    polys = getattr(geom, "geoms", [geom])
    parts = []
    for poly in polys:
        if poly.is_empty:
            continue
        parts.append(ring_d(poly.exterior.coords))
        parts.extend(ring_d(i.coords) for i in poly.interiors)
    return "".join(parts)


def glyph(geom, size, color):
    return f'<path d="{path_d(fit(geom, size))}" fill="{color}" fill-rule="evenodd"/>'


def in_circle(geom, share, ink=INK, paper=PAPER):
    return f'<circle cx="32" cy="32" r="32" fill="{ink}"/>' + glyph(geom, 64 * share, paper)
