import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { cleanProfile, listBrowsers, runningProcesses, scanProfile, siteIcons, type Browser, type CleanError, type CleanReport, type CleanRequest, type Scan } from "./lib/ipc";
import { allTotals, pickedKeys, profileRow, siteRow, sortRows, totals, type Key, type Pick, type Picks, type Row, type Sort } from "./lib/rows";
import { mb } from "./lib/format";
import { Titlebar } from "./components/Titlebar";
import { BrowserNav } from "./components/BrowserNav";
import { LEAVE_MS, SiteRow, type Target } from "./components/SiteRow";
import { TotalsPanel } from "./components/TotalsPanel";
import { CloseDialog } from "./components/CloseDialog";
import { Icon, Kbd } from "./components/Icon";

const ROW = 68;
const GAP = 8;
const STEP = ROW + GAP;
const SORTS: [Sort, string][] = [["fresh", "Сначала свежие"], ["heavy", "Сначала тяжёлые"], ["name", "По имени"]];

type Current = { browser: string; profile: string };
type Cleared = Map<string, Set<Key>>;
const keyOf = (c: Current) => `${c.browser}/${c.profile}`;
const sameSet = (a: Set<string>, b: Set<string>) => a.size === b.size && [...a].every((x) => b.has(x));

export default function App() {
  const [browsers, setBrowsers] = useState<Browser[]>([]);
  const [current, setCurrent] = useState<Current | null>(null);
  const [scans, setScans] = useState<Map<string, Scan | string>>(new Map());
  const [icons, setIcons] = useState<Map<string, Record<string, string>>>(new Map());
  const [scanning, setScanning] = useState<Set<string>>(new Set());
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [picks, setPicks] = useState<Picks>(new Map());
  const [cleared, setCleared] = useState<Cleared>(new Map());
  const [gone, setGone] = useState<Set<string>>(new Set());
  const [leaving, setLeaving] = useState<Set<string>>(new Set());
  const [turbo, setTurbo] = useState(false);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<Sort>("fresh");
  const [asking, setAsking] = useState(false);
  const [toast, setToast] = useState("");
  const [freed, setFreed] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewH, setViewH] = useState(600);
  const listRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const timers = useRef<number[]>([]);
  const currentKey = useRef("");
  const busy = useRef(0);
  const dirty = useRef(new Set<string>());

  const key = current ? keyOf(current) : "";
  currentKey.current = key;
  const browser = browsers.find((b) => b.id === current?.browser);
  const shortName = browser?.name.replace(/^(Google|Microsoft) /, "") ?? "";
  const scan = scans.get(key);
  const siteIconMap = icons.get(key);
  const ready = scan !== undefined && typeof scan !== "string";
  const isScanning = scanning.has(key);
  const isOpen = Boolean(browser && running.has(browser.process));

  const resetWork = useCallback(() => {
    timers.current.forEach((id) => window.clearTimeout(id));
    timers.current = [];
    setPicks(new Map());
    setCleared(new Map());
    setGone(new Set());
    setLeaving(new Set());
  }, []);

  useEffect(() => {
    listBrowsers()
      .then((list) => {
        setBrowsers(list);
        const first = list.find((b) => b.family === "chromium" && b.profiles.length) ?? list.find((b) => b.profiles.length);
        if (first) setCurrent({ browser: first.id, profile: first.profiles[0].id });
      })
      .catch((e) => setToast(`Не удалось найти браузеры: ${e}`));
  }, []);

  useEffect(() => {
    const poll = () =>
      runningProcesses()
        .then((p) => {
          const next = new Set(p);
          setRunning((prev) => (sameSet(prev, next) ? prev : next));
        })
        .catch(() => {});
    poll();
    const id = window.setInterval(poll, 4000);
    return () => window.clearInterval(id);
  }, []);

  const rescan = useCallback(
    (cur: Current) => {
      const k = keyOf(cur);
      dirty.current.delete(k);
      setScanning((s) => new Set(s).add(k));
      scanProfile(cur.browser, cur.profile)
        .then(
          (s) => {
            setScans((m) => new Map(m).set(k, s));
            if (currentKey.current === k) resetWork();
            siteIcons(cur.browser, cur.profile)
              .then((found) => {
                const src = Object.fromEntries(Object.entries(found).map(([d, p]) => [d, convertFileSrc(p)]));
                setIcons((m) => new Map(m).set(k, src));
              })
              .catch(() => {});
          },
          (e) => setScans((m) => new Map(m).set(k, String(e))),
        )
        .finally(() =>
          setScanning((s) => {
            const n = new Set(s);
            n.delete(k);
            return n;
          }),
        );
    },
    [resetWork],
  );

  useEffect(() => {
    if (!current) return;
    resetWork();
    setQuery("");
    const k = keyOf(current);
    if (!scans.has(k) || dirty.current.has(k)) rescan(current);
  }, [current]);

  useEffect(() => {
    listRef.current?.scrollTo({ top: 0 });
    setScrollTop(0);
  }, [current, query, sort]);

  useEffect(() => {
    if (isOpen) setTurbo(false);
  }, [isOpen]);

  useEffect(() => {
    if (!toast) return;
    const id = window.setTimeout(() => setToast(""), Math.max(4500, toast.length * 60));
    return () => window.clearTimeout(id);
  }, [toast]);

  useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    setViewH(el.clientHeight);
    const ro = new ResizeObserver(() => setViewH(el.clientHeight));
    ro.observe(el);
    return () => ro.disconnect();
  }, [ready]);

  useEffect(() => () => timers.current.forEach((id) => window.clearTimeout(id)), []);

  const baseRows = useMemo(() => {
    if (!scan || typeof scan === "string" || !browser) return [];
    const now = new Date();
    return [profileRow(browser.id, shortName, scan), ...scan.sites.map((s) => siteRow(s, now))];
  }, [scan, browser, shortName]);

  const liveRows = useMemo(
    () =>
      baseRows
        .filter((r) => !gone.has(r.id))
        .map((r) => {
          const c = cleared.get(r.id);
          return c ? { ...r, items: r.items.map((i) => (c.has(i.key) ? { ...i, value: 0, short: "" } : i)) } : r;
        }),
    [baseRows, gone, cleared],
  );

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    const sites = sortRows(liveRows.filter((r) => !r.prof && (!q || r.title.includes(q))), sort);
    return q ? sites : [...liveRows.filter((r) => r.prof), ...sites];
  }, [liveRows, query, sort]);

  const t = useMemo(() => totals(liveRows, picks), [liveRows, picks]);
  const all = useMemo(() => allTotals(baseRows), [baseRows]);
  const siteCount = liveRows.filter((r) => !r.prof).length;

  const counts = useMemo(() => {
    const m = new Map<string, number>();
    scans.forEach((s, k) => typeof s !== "string" && m.set(k, s.sites.length));
    if (ready) m.set(key, siteCount);
    return m;
  }, [scans, key, ready, siteCount]);

  const toggle = useCallback((row: Row, target: Target) => {
    setPicks((prev) => {
      const next = new Map(prev);
      const cur = prev.get(row.id);
      if (target === "all") {
        if (cur === "all") next.delete(row.id);
        else next.set(row.id, "all");
        return next;
      }
      const keys = new Set(pickedKeys(row, cur));
      if (keys.has(target)) keys.delete(target);
      else keys.add(target);
      const available = row.items.filter((i) => i.value > 0).length;
      if (!keys.size) next.delete(row.id);
      else next.set(row.id, keys.size === available && available > 1 ? "all" : keys);
      return next;
    });
  }, []);

  const applyClean = useCallback(
    async (targets: Map<string, Pick>, close = false) => {
      if (!current) return;
      const leave: string[] = [];
      const done = new Map<string, Set<Key>>();
      const request: CleanRequest = { sites: {}, profile: [] };
      for (const [id, pick] of targets) {
        const row = liveRows.find((r) => r.id === id);
        if (!row) continue;
        const keys = pickedKeys(row, pick);
        if (!keys.size) continue;
        done.set(id, keys);
        if (row.prof) request.profile = [...keys];
        else request.sites[id] = [...keys];
        if (!row.prof && row.items.every((i) => !i.value || keys.has(i.key))) leave.push(id);
      }
      if (!done.size) return;
      const k = keyOf(current);
      busy.current += 1;
      if (close) setToast(`Закрываю ${shortName}…`);
      let report: CleanReport;
      try {
        report = await cleanProfile(current.browser, current.profile, request, close);
      } catch (e) {
        const err = e as Partial<CleanError>;
        setToast(err.message ?? String(e));
        if (err.touched) {
          dirty.current.add(k);
          if (currentKey.current === k) rescan(current);
        }
        return;
      } finally {
        busy.current -= 1;
      }
      dirty.current.add(k);
      const synced = scan && typeof scan !== "string" && scan.sync && [...done.values()].some((keys) => keys.has("c") || keys.has("h"));
      const head = report.freedBytes > 0 ? `Очищено ${mb(report.freedBytes)}` : "Очищено";
      setToast(synced ? `${head}\nСинхронизация ${shortName} может вернуть историю и входы` : head);
      if (currentKey.current !== k) return;
      setCleared((prev) => {
        const next = new Map(prev);
        done.forEach((keys, id) => next.set(id, new Set([...(prev.get(id) ?? []), ...keys])));
        return next;
      });
      setPicks((prev) => {
        const next = new Map(prev);
        done.forEach((keys, id) => {
          const row = liveRows.find((r) => r.id === id);
          const left = row ? [...pickedKeys(row, prev.get(id))].filter((k) => !keys.has(k)) : [];
          if (left.length) next.set(id, new Set(left));
          else next.delete(id);
        });
        return next;
      });
      setFreed((f) => f + report.freedBytes);
      if (leave.length) {
        setLeaving((s) => new Set([...s, ...leave]));
        const id = window.setTimeout(() => {
          setGone((g) => new Set([...g, ...leave]));
          setLeaving((s) => new Set([...s].filter((x) => !leave.includes(x))));
        }, LEAVE_MS);
        timers.current.push(id);
      }
    },
    [current, liveRows, shortName, rescan, scan],
  );

  const fire = useCallback((row: Row, target: Target) => applyClean(new Map([[row.id, target === "all" ? "all" : new Set([target])]])), [applyClean]);

  const clean = useCallback(() => {
    if (turbo || !t.items || busy.current) return;
    if (isOpen) {
      setAsking(true);
      return;
    }
    applyClean(picks);
  }, [turbo, t.items, isOpen, picks, applyClean]);

  const cancelAsk = useCallback(() => setAsking(false), []);
  const confirmAsk = useCallback(() => {
    setAsking(false);
    applyClean(picks, true);
  }, [applyClean, picks]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (asking) return;
      if (e.ctrlKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchRef.current?.focus();
      } else if (e.key === "Escape") {
        if (picks.size) setPicks(new Map());
        else if (query) setQuery("");
      } else if (e.key === "Delete" && document.activeElement !== searchRef.current) {
        clean();
      } else if (e.key === "F5") {
        e.preventDefault();
        if (current) rescan(current);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [asking, picks, query, clean, current, rescan]);

  const slots = useMemo(() => {
    let count = 0;
    const at = rows.map((r) => (leaving.has(r.id) ? count : count++));
    return { at, count };
  }, [rows, leaving]);
  const first = Math.max(0, Math.floor(scrollTop / STEP) - 4);
  const last = Math.min(rows.length, Math.ceil((scrollTop + viewH) / STEP) + 4);
  const panel = t.items > 0 && !turbo;
  const sortLabel = SORTS.find(([s]) => s === sort)?.[1] ?? "";
  const profileName = browser?.profiles.find((p) => p.id === current?.profile)?.name ?? "";

  return (
    <div className="app">
      <Titlebar path={!browser ? "" : browser.profiles.length > 1 ? `${shortName} · ${profileName}` : shortName} />
      <div className="body" inert={asking || undefined}>
        <BrowserNav browsers={browsers} current={current} counts={counts} running={running} onPick={(b, p) => setCurrent({ browser: b, profile: p })} />
        <section className="panel main" style={{ "--toast-bottom": panel ? "112px" : "16px" } as CSSProperties}>
          <div className="head">
            <h2 className="h">
              {isScanning && !scan ? (
                <>
                  Сканирую <b>{shortName}</b>
                </>
              ) : (
                <>
                  Сайты <b>{ready ? siteCount : "—"}</b>
                </>
              )}
            </h2>
            <span className="meta">
              {turbo && freed > 0 ? (
                <>
                  за сеанс очищено <b>{mb(freed)}</b>
                </>
              ) : isScanning && scan ? (
                "обновляю…"
              ) : (
                ""
              )}
            </span>
            <button
              type="button"
              className={`turbo${turbo ? " on" : ""}`}
              disabled={isOpen || !ready}
              onClick={() => {
                if (!turbo) setPicks(new Map());
                setTurbo(!turbo);
              }}
              title={isOpen ? `${shortName} открыт — турбо после закрытия` : "Турбо: держи пункт, чтобы очистить сразу"}
              aria-pressed={turbo}
            >
              <Icon name="zap" />
              Турбо
              <span className="sw" />
            </button>
            <button type="button" className="round" aria-label="Обновить" title="Обновить — F5" onClick={() => current && rescan(current)}>
              <Icon name="refresh" />
            </button>
          </div>

          {isScanning && !scan && (
            <>
              <div className="progress">
                <i />
              </div>
              <div className="list">
                {Array.from({ length: 7 }, (_, i) => (
                  <div key={i} className="skel" />
                ))}
              </div>
            </>
          )}

          {typeof scan === "string" && (
            <div className="empty">
              <h3>
                <b>{shortName}</b> пока не читается
              </h3>
              <span>{scan}</span>
            </div>
          )}

          {ready && (
            <>
              {scan.locked.length > 0 && (
                <div className="note">
                  <Icon name="lock" />
                  <span>
                    <b>{scan.locked.join(", ")}</b> заняты {shortName}. Прочитаю, если закрыть {shortName} или запустить Cache Out от администратора.
                  </span>
                </div>
              )}
              <div className="tools">
                <label className="search">
                  <Icon name="search" />
                  <input ref={searchRef} value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Найти сайт" aria-label="Найти сайт" spellCheck={false} />
                  <Kbd k="Ctrl F" />
                </label>
                <button type="button" className="sort" onClick={() => setSort(SORTS[(SORTS.findIndex(([s]) => s === sort) + 1) % SORTS.length][0])}>
                  {sortLabel}
                  <Icon name="down" />
                </button>
              </div>
              <div className="list" ref={listRef} onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}>
                {rows.length === 0 ? (
                  <div className="empty">
                    <h3>
                      Сайтов с «<b>{query}</b>» нет
                    </h3>
                    <span>Поиск идёт по домену.</span>
                  </div>
                ) : (
                  <div className={`list-inner${leaving.size ? " shifting" : ""}`} style={{ height: slots.count * STEP + (panel ? 110 : 64) }}>
                    {rows.slice(first, last).map((row, i) => (
                      <SiteRow key={row.id} row={row} pick={picks.get(row.id)} turbo={turbo} top={slots.at[first + i] * STEP} leaving={leaving.has(row.id)} icon={siteIconMap?.[row.id]} onToggle={toggle} onFire={fire} />
                    ))}
                  </div>
                )}
              </div>
            </>
          )}

          {panel && <TotalsPanel t={t} all={all} browserName={shortName} onClean={clean} />}
          {toast && (
            <div className="toast" role="status">
              {toast.split("\n").map((line, i) => (
                <span key={i}>{line}</span>
              ))}
            </div>
          )}
        </section>
      </div>
      {asking && <CloseDialog browserName={shortName} items={t.items} onCancel={cancelAsk} onConfirm={confirmAsk} />}
    </div>
  );
}
