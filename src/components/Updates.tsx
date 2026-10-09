import { checkNow, installNow, useShip } from "@stillmvd/tauri-ship";
import { mb, plural } from "../lib/format";

function checkedAgo(ms: number) {
  const min = Math.floor((Date.now() - ms) / 60_000);
  if (min < 1) return "проверено только что";
  if (min < 60) return `проверено ${min} ${plural(min, ["минуту", "минуты", "минут"])} назад`;
  return `проверено в ${new Date(ms).toLocaleTimeString("ru-RU", { hour: "2-digit", minute: "2-digit" })}`;
}

export function Updates() {
  const { status } = useShip();
  const phase = status?.phase ?? "idle";
  const available = status?.available ?? null;
  const hint =
    phase === "checking"
      ? "Проверяю…"
      : status?.lastCheck
        ? checkedAgo(status.lastCheck) + (available || status.error ? "" : " · это последняя версия")
        : "Ещё не проверял";
  const progress =
    phase === "downloading"
      ? status?.downloaded
        ? `Скачиваю: ${mb(status.downloaded)}${status.total ? ` из ${mb(status.total)}` : ""}`
        : "Скачиваю…"
      : phase === "installing"
        ? "Ставлю — приложение закроется и откроется заново"
        : phase === "ready"
          ? "Скачана, поставится при перезапуске"
          : "Скачается в фоне";

  return (
    <div className="ups" role="dialog" aria-label="Обновления">
      <div className="row">
        <div className="tx">
          <b>Версия {status?.current ?? ""}</b>
          <span>{hint}</span>
        </div>
        <button type="button" className="btn sec" disabled={phase !== "idle"} aria-busy={phase === "checking"} onClick={() => void checkNow().catch(() => {})}>
          Проверить
        </button>
      </div>
      {available && (
        <div className="row">
          <div className="tx">
            <b>Доступна {available.version}</b>
            <span aria-live="polite">{progress}</span>
          </div>
          {(phase === "ready" || phase === "installing") && (
            <button type="button" className="btn" disabled={phase === "installing"} aria-busy={phase === "installing"} onClick={() => void installNow().catch(() => {})}>
              Перезапустить для обновления
            </button>
          )}
        </div>
      )}
      {status?.error && <p className="err">{status.error}</p>}
    </div>
  );
}
