import type { MeetingProgress } from './api';
import { translate, type Lang } from './i18n';

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

/** Local calendar day of an ISO date, as `YYYY-MM-DD`. */
export function localDay(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** Groups items by local calendar day, days in first-seen order, items in their order. */
export function groupByDay<T>(items: T[], iso: (item: T) => string): { day: string; items: T[] }[] {
  const days = new Map<string, T[]>();
  for (const item of items) {
    const day = localDay(iso(item));
    days.set(day, [...(days.get(day) ?? []), item]);
  }
  return [...days].map(([day, items]) => ({ day, items }));
}

/** A `YYYY-MM-DD` day as a long localized date, e.g. "mardi 20 octobre". */
export function formatDay(day: string, lang: string): string {
  const [y, m, d] = day.split('-').map(Number);
  return new Intl.DateTimeFormat(lang, { weekday: 'long', day: 'numeric', month: 'long' }).format(new Date(y, m - 1, d));
}

/** "Étape 2 sur 5 : séparation des voix". */
export function progressLabel(p: MeetingProgress, lang: Lang): string {
  return translate(lang, 'progress.step', { n: p.index, total: p.total, label: translate(lang, `progress.${p.step}`) });
}
