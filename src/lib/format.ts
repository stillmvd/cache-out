export const plural = (n: number, [one, few, many]: [string, string, string]) => {
  const m10 = n % 10;
  const m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return one;
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few;
  return many;
};

export const nf = (n: number) => String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, " ");

const MB = 1024 * 1024;
export const mb = (bytes: number) => `${(bytes / MB).toFixed(1).replace(".", ",")} МБ`;
export const mbShort = (bytes: number) => {
  const v = bytes / MB;
  if (v >= 1024) return `${(v / 1024).toFixed(1).replace(".", ",")} ГБ`;
  if (v >= 10) return `${Math.round(v)} МБ`;
  if (v >= 0.1) return `${v.toFixed(1).replace(".", ",")} МБ`;
  return `${Math.max(1, Math.round(bytes / 1024))} КБ`;
};

export const when = (ms: number | null, now = new Date()) => {
  if (!ms) return "не открывался";
  const d = new Date(ms);
  const hm = d.toLocaleTimeString("ru-RU", { hour: "2-digit", minute: "2-digit" });
  const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((day(now) - day(d)) / 86_400_000);
  if (diff === 0) return `сегодня ${hm}`;
  if (diff === 1) return `вчера ${hm}`;
  return d.toLocaleDateString("ru-RU", { day: "numeric", month: "short", year: d.getFullYear() === now.getFullYear() ? undefined : "numeric" }).replace(".", "");
};
