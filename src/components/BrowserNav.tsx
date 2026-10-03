import type { Browser } from "../lib/ipc";
import { BrowserGlyph } from "./Icon";

type Props = {
  browsers: Browser[];
  current: { browser: string; profile: string } | null;
  counts: Map<string, number>;
  running: Set<string>;
  onPick: (browser: string, profile: string) => void;
};

export function BrowserNav({ browsers, current, counts, running, onPick }: Props) {
  return (
    <nav className="panel nav" aria-label="Браузеры">
      <span className="cap">Браузеры</span>
      {browsers.map((b) => (
        <div key={b.id} style={{ display: "contents" }}>
          <div className="br">
            <BrowserGlyph id={b.id} />
            {b.name.replace(/^(Google|Microsoft) /, "")}
            {running.has(b.process) && (
              <span className="dot">
                <i />
                открыт
              </span>
            )}
          </div>
          {b.profiles.map((p) => {
            const key = `${b.id}/${p.id}`;
            const on = current?.browser === b.id && current.profile === p.id;
            return (
              <button key={key} type="button" className={`pr${on ? " on" : ""}`} onClick={() => onPick(b.id, p.id)} aria-current={on || undefined}>
                <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{p.name}</span>
                {counts.has(key) && <span className="n">{counts.get(key)}</span>}
              </button>
            );
          })}
        </div>
      ))}
    </nav>
  );
}
