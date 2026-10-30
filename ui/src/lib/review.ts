import type { MeetingState, Participant, ReviewItem, Segment } from './api';

/** Validated meetings (sent, being sent, or whose send failed) open read-only, with an export. */
export function isReadOnly(state: MeetingState, error: string | null): boolean {
  return state === 'reviewed' || state === 'sending' || state === 'sent' || (state === 'failed' && !!error?.startsWith('send_failed'));
}

export const isOpen = (i: ReviewItem) => i.resolution === null;

export function remaining(items: ReviewItem[]): number {
  return items.filter(isOpen).length;
}

/** Rule: a meeting is sent only once every item is resolved ("Ignorer" is one keystroke away). */
export function canSend(items: ReviewItem[]): boolean {
  return remaining(items) === 0;
}

/** j/k navigation over the list order, clamped at both ends. */
export function stepId(items: ReviewItem[], current: string | null, dir: 1 | -1): string | null {
  if (items.length === 0) return null;
  const i = items.findIndex((x) => x.id === current);
  if (i === -1) return (dir === 1 ? items[0] : items[items.length - 1]).id;
  return items[Math.min(items.length - 1, Math.max(0, i + dir))].id;
}

/** After resolving `id`, the next open item below it, else the first open one. */
export function nextOpenAfter(items: ReviewItem[], id: string): string | null {
  const i = items.findIndex((x) => x.id === id);
  const after = items.slice(i + 1).find(isOpen) ?? items.find((x) => isOpen(x) && x.id !== id);
  return after?.id ?? null;
}

export function flaggedSegments(items: ReviewItem[]): Set<string> {
  return new Set(items.filter(isOpen).flatMap((i) => (i.segment_id ? [i.segment_id] : [])));
}

export function segmentAt(segments: Segment[], time: number): Segment | null {
  return segments.find((s) => time >= s.start && time < s.end) ?? null;
}

export function speakerIds(segments: Segment[]): string[] {
  return [...new Set(segments.flatMap((s) => (s.speaker_id ? [s.speaker_id] : [])))];
}

export function speakerName(
  id: string | null,
  participants: Participant[],
  t: (key: string, params?: Record<string, string>) => string,
): string {
  if (id === null) return t('speaker.unknown');
  const p = participants.find((x) => x.id === id);
  // "Moi" is the backend's unnamed default, shown in the interface language.
  if (id === 'me') return p && p.name !== 'Moi' ? p.name : t('speaker.me');
  if (p) return p.name;
  const anon = /^speaker_([a-z])$/.exec(id);
  return anon ? t('speaker.anon', { letter: anon[1].toUpperCase() }) : id;
}

/** The alternative cut into parts, `changed` on the words that differ from the proposed text (common start and end kept plain). */
export function wordDiff(base: string, alt: string): { text: string; changed: boolean }[] {
  const a = base.split(/(\s+)/);
  const b = alt.split(/(\s+)/);
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start++;
  let end = b.length;
  while (end > start && a.length - (b.length - end) > start && a[a.length - 1 - (b.length - end)] === b[end - 1]) end--;
  // Spaces at the edges of the change stay plain.
  if (start < end && /^\s+$/.test(b[start])) start++;
  if (start < end && /^\s+$/.test(b[end - 1])) end--;
  const parts = [
    { text: b.slice(0, start).join(''), changed: false },
    { text: b.slice(start, end).join(''), changed: true },
    { text: b.slice(end).join(''), changed: false },
  ];
  return parts.filter((p) => p.text);
}
