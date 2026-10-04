import { bytes, type MODES, type Totals } from "../lib/rows";
import { mb, nf, plural } from "../lib/format";
import { Icon, Kbd } from "./Icon";

const pct = (v: number, of: number) => (of > 0 ? Math.min(100, Math.max(v > 0 ? 3 : 0, Math.round((v / of) * 100))) : 0);

export function TotalsPanel({ t, all, browserName, onClean }: { t: Totals; all: Totals; browserName: string; onClean: () => void }) {
  const cells = [
    ["Куки", "cookie", t.c, nf(t.c), all.c],
    ["Визиты", "history", t.h, nf(t.h), all.h],
    ["Загрузки", "download", t.d, nf(t.d), all.d],
    ["Хранилище", "storage", t.s, mb(t.s), all.s],
    ["Кеш", "layers", t.cache, mb(t.cache), all.cache],
  ] as const;
  const where = [t.sites ? `${t.sites} ${plural(t.sites, ["сайт", "сайта", "сайтов"])}` : "", t.prof ? browserName : ""].filter(Boolean).join(" и ");
  return (
    <div className="pn" role="region" aria-label="Итог выбора">
      {cells.map(([label, icon, v, text, of]) => (
        <div key={label} className={`cat${v ? "" : " z"}`}>
          <span className="t">
            <Icon name={icon} />
            <span>{label}</span>
          </span>
          <b>{v ? text : "—"}</b>
          <span className="meter">
            <i style={{ width: `${pct(v, of)}%` }} />
          </span>
        </div>
      ))}
      <div className="sum">
        <b>{mb(t.bytes)}</b>
        <span>
          {t.items} {plural(t.items, ["пункт", "пункта", "пунктов"])} · {where}
        </span>
      </div>
      <button type="button" className="btn" onClick={onClean}>
        Очистить <Kbd k="Del" />
      </button>
    </div>
  );
}

type ModeTotals = { value: number; all: number; sites: number; of: number };

export function ModePanel({ mode, t, onClean }: { mode: (typeof MODES)[number]; t: ModeTotals; onClean: () => void }) {
  const fmt = (v: number) => (bytes(mode.key) ? mb(v) : nf(v));
  return (
    <div className="pn" role="region" aria-label="Итог выбора">
      <div className="cat">
        <span className="t">
          <Icon name={mode.icon} />
          <span>{mode.label}</span>
        </span>
        <b>
          {fmt(t.value)} <span className="of">из {fmt(t.all)}</span>
        </b>
        <span className="meter">
          <i style={{ width: `${pct(t.value, t.all)}%` }} />
        </span>
      </div>
      <div className="sum">
        <b>
          {t.sites} из {t.of}
        </b>
        <span>{plural(t.of, ["сайта", "сайтов", "сайтов"])} выбрано</span>
      </div>
      <button type="button" className="btn" onClick={onClean}>
        Очистить {mode.what} <Kbd k="Del" />
      </button>
    </div>
  );
}
