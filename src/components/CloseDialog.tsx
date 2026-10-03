import { useEffect, useRef } from "react";
import { plural } from "../lib/format";
import { Kbd } from "./Icon";

type Props = { browserName: string; items: number; onCancel: () => void; onConfirm: () => void };

export function CloseDialog({ browserName, items, onCancel, onConfirm }: Props) {
  const confirm = useRef<HTMLButtonElement>(null);
  const cancelRef = useRef(onCancel);
  cancelRef.current = onCancel;
  useEffect(() => {
    const back = document.activeElement as HTMLElement | null;
    confirm.current?.focus({ preventScroll: true });
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancelRef.current();
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      back?.focus?.({ preventScroll: true });
    };
  }, []);
  return (
    <div className="scrim" onPointerDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="dlg" role="dialog" aria-modal="true" aria-labelledby="close-title" aria-describedby="close-text">
        <h3 id="close-title">
          Закрыть <b>{browserName}</b>?
        </h3>
        <p id="close-text">
          Пока {browserName} открыт, он держит файлы профиля. Закрою его, очищу {items} {plural(items, ["пункт", "пункта", "пунктов"])} и не буду запускать снова. Вкладки
          вернутся, если в {browserName} включено «Продолжить с того же места».
        </p>
        <div className="acts">
          <button type="button" className="btn sec" onClick={onCancel}>
            Отмена <Kbd k="Esc" />
          </button>
          <button type="button" className="btn" ref={confirm} onClick={onConfirm}>
            Закрыть и очистить <Kbd k="Enter" />
          </button>
        </div>
      </div>
    </div>
  );
}
