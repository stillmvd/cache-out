import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { updateInstall, updateReady } from "../lib/ipc";
import { Icon, Mark } from "./Icon";

export function Titlebar({ path, onError }: { path: string; onError: (message: string) => void }) {
  const [update, setUpdate] = useState(false);
  useEffect(() => {
    updateReady().then(setUpdate, () => {});
    const off = listen("update://ready", () => setUpdate(true));
    return () => void off.then((f) => f());
  }, []);
  return (
    <header className="tb" data-tauri-drag-region>
      <Mark />
      <b data-tauri-drag-region>Cache Out</b>
      <span className="path" data-tauri-drag-region>{path}</span>
      <div className="wb">
        {update && (
          <button type="button" className="upd" aria-label="Обновить и перезапустить" title="Обновить и перезапустить" onClick={() => updateInstall().catch((e) => onError(String(e)))}>
            <Icon name="update" />
          </button>
        )}
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
