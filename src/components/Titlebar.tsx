import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { UpdateBadge } from "@stillmvd/tauri-ship";
import { Icon, Mark } from "./Icon";
import { Updates } from "./Updates";

export function Titlebar({ path }: { path: string }) {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", down);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", down);
      document.removeEventListener("keydown", key);
    };
  }, [open]);
  return (
    <header className="tb" data-tauri-drag-region>
      <Mark />
      <b data-tauri-drag-region>Cache Out</b>
      <span className="path" data-tauri-drag-region>{path}</span>
      <div className="wb">
        <UpdateBadge lang="ru" />
        <div className="upw" ref={box}>
          <button type="button" aria-label="Обновления" title="Обновления" aria-expanded={open} onClick={() => setOpen((v) => !v)}>
            <Icon name="refresh" />
          </button>
          {open && <Updates />}
        </div>
        <button type="button" aria-label="Свернуть" onClick={() => getCurrentWindow().minimize()}>
          <Icon name="min" />
        </button>
        <button type="button" aria-label="Развернуть" onClick={() => getCurrentWindow().toggleMaximize()}>
          <Icon name="max" />
        </button>
        <button type="button" className="x" aria-label="Закрыть" onClick={() => getCurrentWindow().close()}>
          <Icon name="close" />
        </button>
      </div>
    </header>
  );
}
