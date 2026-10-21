import type { Participant, ReviewItem, Segment } from './api';

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
  if (id === 'me') return t('speaker.me');
  const p = participants.find((x) => x.id === id);
  if (p) return p.name;
  const anon = /^speaker_([a-z])$/.exec(id);
  return anon ? t('speaker.anon', { letter: anon[1].toUpperCase() }) : id;
}
