import type { Scan, Site } from "./ipc";
import { mbShort, nf, plural, when, mb } from "./format";

export type Key = "c" | "h" | "d" | "s" | "k" | "bc" | "f" | "a";
export type Item = { key: Key; label: string; icon: string; value: number; short: string };
export type Row = { id: string; title: string; sub: string; prof: boolean; glyph?: string; items: Item[]; weight: number; last: number | null };
export type Pick = Set<Key> | "all";
export type Picks = Map<string, Pick>;
export type Sort = "fresh" | "heavy" | "name";

export type Mode = "c" | "s" | "k" | "d" | "h";

export const PROFILE_ROW = "__profile__";
export const MODES: { key: Mode; label: string; icon: string; with: string; what: string }[] = [
  { key: "c", label: "Куки", icon: "cookie", with: "с куки", what: "куки" },
  { key: "s", label: "Хранилище", icon: "storage", with: "с хранилищем", what: "хранилище" },
  { key: "k", label: "Кеш сайта", icon: "layers", with: "с кешем", what: "кеш" },
  { key: "d", label: "Загрузки", icon: "download", with: "с загрузками", what: "загрузки" },
  { key: "h", label: "История", icon: "history", with: "с историей", what: "историю" },
];
export const bytes = (k: Key) => k === "s" || k === "k" || k === "bc";
export const valueOf = (row: Row, k: Key) => row.items.find((i) => i.key === k)?.value ?? 0;
export const unitOf = (k: Key, v: number) =>
  k === "c" ? "куки" : k === "h" ? plural(v, ["визит", "визита", "визитов"]) : k === "d" ? plural(v, ["загрузка", "загрузки", "загрузок"]) : "";
const shortOf = (k: Key, v: number) => (bytes(k) ? mbShort(v) : nf(v));

function item(key: Key, label: string, icon: string, value: number): Item {
  return { key, label, icon, value, short: value > 0 ? shortOf(key, value) : "" };
}

export function siteRow(x: Site, now: Date): Row {
  const visits = x.historyUrls > 0 ? Math.max(x.visits, x.historyUrls) : 0;
  const weight = x.storageBytes + x.siteCacheBytes;
  const parts = [when(x.lastVisit, now)];
  if (visits) parts.push(`${nf(visits)} ${plural(visits, ["визит", "визита", "визитов"])}`);
  if (weight >= 100 * 1024) parts.push(mb(weight));
  return {
    id: x.domain,
    title: x.domain,
    sub: parts.join(" · "),
    prof: false,
    weight,
    last: x.lastVisit ?? x.lastCookieAccess,
    items: [
      item("c", "Куки", "cookie", x.cookies),
      item("h", "История", "history", visits),
      item("d", "Загрузки", "download", x.downloads),
      item("s", "Хранилище", "storage", x.storageBytes),
      item("k", "Кеш сайта", "layers", x.siteCacheBytes),
    ],
  };
}

export function profileRow(browserId: string, browserName: string, scan: Scan): Row {
  const forms = scan.forms.reduce((a, f) => a + f.entries, 0);
  return {
    id: PROFILE_ROW,
    title: `${browserName} целиком`,
    sub: "Кеш браузера не делится по сайтам · формы · адреса",
    prof: true,
    glyph: browserId,
    weight: scan.cacheBytes,
    last: null,
    items: [
      item("bc", "Кеш браузера", "cache", scan.cacheBytes),
      item("f", "Формы", "form", forms),
      item("a", "Адреса", "pin", scan.addresses),
    ],
  };
}

export function sortRows(rows: Row[], sort: Sort, mode: Mode | null = null) {
  const r = [...rows];
  const w = (x: Row) => (mode ? valueOf(x, mode) : x.weight);
  if (sort === "fresh") r.sort((a, b) => (b.last ?? 0) - (a.last ?? 0) || w(b) - w(a));
  if (sort === "heavy") r.sort((a, b) => w(b) - w(a) || (b.last ?? 0) - (a.last ?? 0));
  if (sort === "name") r.sort((a, b) => a.title.localeCompare(b.title));
  return r;
}

export function pickedKeys(row: Row, pick: Pick | undefined): Set<Key> {
  if (!pick) return new Set();
  if (pick === "all") return new Set(row.items.filter((i) => i.value > 0).map((i) => i.key));
  return pick;
}

export type Totals = { items: number; sites: number; prof: boolean; c: number; h: number; d: number; s: number; cache: number; bytes: number };

export function totals(rows: Row[], picks: Picks): Totals {
  const t: Totals = { items: 0, sites: 0, prof: false, c: 0, h: 0, d: 0, s: 0, cache: 0, bytes: 0 };
  for (const row of rows) {
    const keys = pickedKeys(row, picks.get(row.id));
    if (!keys.size) continue;
    t.items += keys.size;
    if (row.prof) t.prof = true;
    else t.sites += 1;
    for (const i of row.items) {
      if (!keys.has(i.key)) continue;
      if (i.key === "c") t.c += i.value;
      if (i.key === "h") t.h += i.value;
      if (i.key === "d") t.d += i.value;
      if (i.key === "s") t.s += i.value;
      if (i.key === "k" || i.key === "bc") t.cache += i.value;
    }
  }
  t.bytes = t.s + t.cache;
  return t;
}

export function allTotals(rows: Row[]): Totals {
  const picks: Picks = new Map(rows.map((r) => [r.id, "all" as const]));
  return totals(rows, picks);
}
