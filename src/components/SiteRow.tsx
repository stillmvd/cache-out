import { memo, useEffect, useRef, useState, type CSSProperties } from "react";
import { pickedKeys, type Item, type Key, type Pick, type Row } from "../lib/rows";
import { BrowserGlyph, Icon } from "./Icon";

export const HOLD_MS = 600;
export const LEAVE_MS = 700;
export type Target = Key | "all";

type CubeProps = {
  icon: string;
  short: string;
  title: string;
  state: "" | "on" | "via";
  disabled: boolean;
  turbo: boolean;
  onToggle: () => void;
  onFire: () => void;
};

function Cube({ icon, short, title, state, disabled, turbo, onToggle, onFire }: CubeProps) {
  const [holding, setHolding] = useState(false);
  const timer = useRef(0);
  const live = useRef({ turbo, disabled, onFire });
  live.current = { turbo, disabled, onFire };
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const start = () => {
    if (!turbo || disabled) return;
    window.clearTimeout(timer.current);
    setHolding(true);
    timer.current = window.setTimeout(() => {
      setHolding(false);
      if (live.current.turbo && !live.current.disabled) live.current.onFire();
    }, HOLD_MS);
  };
  const cancel = () => {
    window.clearTimeout(timer.current);
    setHolding(false);
  };
  return (
    <button
      type="button"
      className={`cube ${state}${holding ? " holding" : ""}`}
      style={{ "--hold": `${HOLD_MS}ms` } as CSSProperties}
      disabled={disabled}
      title={title}
      aria-label={title}
      aria-pressed={turbo ? undefined : state !== ""}
      onClick={() => !turbo && onToggle()}
      onPointerDown={(e) => e.button === 0 && e.isPrimary && start()}
      onPointerUp={cancel}
      onPointerLeave={cancel}
      onPointerCancel={cancel}
      onBlur={cancel}
      onKeyDown={(e) => {
        if (turbo && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          if (!e.repeat) start();
        }
      }}
      onKeyUp={(e) => (e.key === "Enter" || e.key === " ") && cancel()}
    >
      <Icon name={icon} />
      {short && <b>{short}</b>}
    </button>
  );
}

const BITS = [[90, 20, 12, 8, -46, 40], [84, 50, 9, 12, -30, -30], [76, 15, 14, 18, -54, 60], [68, 55, 8, 26, -40, 20], [58, 30, 11, 30, -60, -50], [48, 45, 7, 34, -44, 30], [36, 20, 10, 38, -64, 45], [24, 50, 9, 40, -40, -20], [12, 25, 12, 42, -58, 35]];

function Bits() {
  return (
    <span className="bits">
      {BITS.map(([x, y, s, tx, ty, r], i) => (
        <i key={i} style={{ "--x": `${x}%`, "--y": `${y}%`, "--s": `${s}px`, "--tx": `${tx}px`, "--ty": `${ty}px`, "--r": `${r}deg`, "--d": `${i * 25}ms` } as CSSProperties} />
      ))}
    </span>
  );
}

const itemTitle = (row: Row, i: Item, hold: string) =>
  i.value ? `${i.label}: ${i.short}${hold ? ` — ${hold}` : ""}` : `${i.label}: у ${row.prof ? "браузера" : "сайта"} нет`;

type Props = {
  row: Row;
  pick: Pick | undefined;
  turbo: boolean;
  hold: string;
  top: number;
  leaving: boolean;
  icon?: string;
  onToggle: (row: Row, target: Target) => void;
  onFire: (row: Row, target: Target) => void;
  onOpen: (row: Row) => void;
};

function Fav({ row, icon }: { row: Row; icon?: string }) {
  const [broken, setBroken] = useState("");
  if (row.prof) return <BrowserGlyph id={row.glyph ?? "chrome"} />;
  if (icon && broken !== icon) return <img src={icon} alt="" draggable={false} onError={() => setBroken(icon)} />;
  return <>{row.title[0]}</>;
}

export const SiteRow = memo(function SiteRow({ row, pick, turbo, hold, top, leaving, icon, onToggle, onFire, onOpen }: Props) {
  const keys = pickedKeys(row, pick);
  const all = pick === "all";
  const empty = row.items.every((i) => !i.value);
  return (
    <div className={`rw${row.prof ? " prof" : ""}${leaving ? " leaving" : ""}`} style={{ transform: `translateY(${top}px)` }}>
      <span className="who">
        <span className="fav">
          <Fav row={row} icon={icon} />
        </span>
        <span className="txt">
          {row.prof ? (
            <span className="dom">{row.title}</span>
          ) : (
            <button type="button" className="dom link" onClick={() => onOpen(row)} title={`Открыть ${row.title} в браузере`}>
              <span>{row.title}</span>
              <Icon name="ext" />
            </button>
          )}
          <span className="sub">{row.sub}</span>
        </span>
      </span>
      <span className="chips">
        {row.items.map((i) => (
          <Cube
            key={i.key}
            icon={i.icon}
            short={i.short}
            title={itemTitle(row, i, turbo ? hold : "")}
            state={keys.has(i.key) ? (all ? "via" : "on") : ""}
            disabled={!i.value || leaving}
            turbo={turbo}
            onToggle={() => onToggle(row, i.key)}
            onFire={() => onFire(row, i.key)}
          />
        ))}
        <Cube
          icon="all"
          short="Всё"
          title={turbo ? `Всё у ${row.title} — ${hold}` : `Всё у ${row.title}`}
          state={all ? "on" : ""}
          disabled={empty || leaving}
          turbo={turbo}
          onToggle={() => onToggle(row, "all")}
          onFire={() => onFire(row, "all")}
        />
      </span>
      {leaving && <Bits />}
    </div>
  );
});
