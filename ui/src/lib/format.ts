/** `mm:ss`, or `h:mm:ss` from one hour. Negative or invalid input reads as 0. */
export function formatTimestamp(seconds: number): string {
  const total = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mmss = `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return h > 0 ? `${h}:${mmss}` : mmss;
}

export function formatDateTime(iso: string, lang: string): string {
  return new Intl.DateTimeFormat(lang, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }).format(new Date(iso));
}

export function formatTime(iso: string, lang: string): string {
  return new Intl.DateTimeFormat(lang, { hour: '2-digit', minute: '2-digit' }).format(new Date(iso));
}

/** Size in MB to a short localized number, switching to GB from 1000 MB. */
export function formatSize(mb: number, lang: string): { unit: 'mb' | 'gb'; n: string } {
  if (mb < 1000) return { unit: 'mb', n: new Intl.NumberFormat(lang, { maximumFractionDigits: 0 }).format(mb) };
  return { unit: 'gb', n: new Intl.NumberFormat(lang, { maximumFractionDigits: 1 }).format(mb / 1000) };
}
