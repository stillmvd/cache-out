import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon, Mark } from "./Icon";

export function Titlebar({ path }: { path: string }) {
  return (
    <header className="tb" data-tauri-drag-region>
      <Mark />
      <b data-tauri-drag-region>Cache Out</b>
      <span className="path" data-tauri-drag-region>{path}</span>
      <div className="wb">
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
