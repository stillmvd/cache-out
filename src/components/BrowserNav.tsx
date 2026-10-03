import type { Browser, Profile } from "../lib/ipc";
import { BrowserGlyph } from "./Icon";

type Props = {
  browsers: Browser[];
  current: { browser: string; profile: string } | null;
  counts: Map<string, number>;
  running: Set<string>;
  onPick: (browser: string, profile: string) => void;
};

function Avatar({ p }: { p: Profile }) {
  return p.avatar ? <img className="av" src={p.avatar} alt="" draggable={false} /> : <span className="av">{p.name[0]?.toUpperCase()}</span>;
}

export function BrowserNav({ browsers, current, counts, running, onPick }: Props) {
  return (
    <nav className="panel nav" aria-label="Браузеры">
      <span className="cap">Браузеры</span>
      {browsers.map((b) => {
        const name = b.name.replace(/^(Google|Microsoft) /, "");
        const open = running.has(b.process) && (
          <span className="dot">
            <i />
            открыт
          </span>
        );
        if (b.profiles.length === 1) {
          const p = b.profiles[0];
          const key = `${b.id}/${p.id}`;
          const on = current?.browser === b.id && current.profile === p.id;
          return (
            <button key={b.id} type="button" className={`br solo${on ? " on" : ""}`} onClick={() => onPick(b.id, p.id)} aria-current={on || undefined}>
              <BrowserGlyph id={b.id} />
              {name}
              {open}
              {counts.has(key) && <span className="n">{counts.get(key)}</span>}
            </button>
          );
        }
        return (
          <div key={b.id} style={{ display: "contents" }}>
            <div className="br">
              <BrowserGlyph id={b.id} />
              {name}
              {open}
            </div>
            {b.profiles.map((p) => {
              const key = `${b.id}/${p.id}`;
              const on = current?.browser === b.id && current.profile === p.id;
              return (
                <button key={key} type="button" className={`pr${on ? " on" : ""}`} onClick={() => onPick(b.id, p.id)} aria-current={on || undefined}>
                  <Avatar p={p} />
                  <span className="pn-name">{p.name}</span>
                  {counts.has(key) && <span className="n">{counts.get(key)}</span>}
                </button>
              );
            })}
          </div>
        );
      })}
    </nav>
  );
}
