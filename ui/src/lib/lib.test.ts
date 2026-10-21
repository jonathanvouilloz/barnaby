import { describe, expect, test } from 'bun:test';
import { translate, dictionaries } from './i18n';
import { acceleratorFromKey, type KeyLike } from './accelerator';
import { formatTimestamp, formatSize } from './format';
import { canSend, nextOpenAfter, remaining, segmentAt, speakerName, stepId, flaggedSegments } from './review';
import { addTerm, isLoopbackUrl } from './settings';
import type { ModelInfo, ReviewItem, Segment, Settings } from './api';
import { missingRequired } from './models';

describe('i18n', () => {
  test('replaces params and keeps unknown placeholders', () => {
    expect(translate('fr', 'review.progress', { done: 1, total: 4 })).toBe('1 sur 4 traitées');
    expect(translate('en', 'review.progress', { done: 1 })).toBe('1 of {total} resolved');
  });
  test('falls back to French, then to the key', () => {
    dictionaries.fr['test.only_fr'] = 'Bonjour';
    expect(translate('en', 'test.only_fr')).toBe('Bonjour');
    expect(translate('en', 'missing.key')).toBe('missing.key');
  });
  test('English covers every French key', () => {
    const missing = Object.keys(dictionaries.fr).filter((k) => !k.startsWith('test.') && !(k in dictionaries.en));
    expect(missing).toEqual([]);
  });
  test('review questions and capture warnings exist for every kind', () => {
    for (const k of ['low_confidence', 'overlap', 'unattributed_speaker', 'uncertain_term', 'inaudible'])
      expect(translate('fr', `review.q.${k}`)).not.toBe(`review.q.${k}`);
    for (const k of ['system_silent', 'no_mic', 'no_system', 'permission_denied'])
      expect(translate('en', `capture.${k}`)).not.toBe(`capture.${k}`);
  });
});

const key = (code: string, mods: Partial<KeyLike> = {}, k = code): KeyLike => ({
  key: k, code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods,
});

describe('accelerator', () => {
  test('builds Tauri strings in a fixed modifier order', () => {
    expect(acceleratorFromKey(key('Space', { ctrlKey: true, altKey: true }, ' '))).toBe('Ctrl+Alt+Space');
    expect(acceleratorFromKey(key('KeyR', { shiftKey: true, ctrlKey: true }, 'R'))).toBe('Ctrl+Shift+R');
    expect(acceleratorFromKey(key('Digit3', { metaKey: true }, '3'))).toBe('Super+3');
  });
  test('waits while only modifiers are held', () => {
    expect(acceleratorFromKey(key('ControlLeft', { ctrlKey: true }, 'Control'))).toBeNull();
  });
  test('refuses bare keys but allows function keys', () => {
    expect(acceleratorFromKey(key('KeyA', {}, 'a'))).toBe('need_modifier');
    expect(acceleratorFromKey(key('F9'))).toBe('F9');
  });
  test('uses the physical key, not the layout', () => {
    // AZERTY: the key labelled A sits on code KeyQ
    expect(acceleratorFromKey(key('KeyQ', { ctrlKey: true }, 'a'))).toBe('Ctrl+Q');
  });
});

describe('format', () => {
  test('timestamps', () => {
    expect(formatTimestamp(0)).toBe('00:00');
    expect(formatTimestamp(65.9)).toBe('01:05');
    expect(formatTimestamp(3725)).toBe('1:02:05');
    expect(formatTimestamp(-3)).toBe('00:00');
    expect(formatTimestamp(NaN)).toBe('00:00');
  });
  test('sizes', () => {
    expect(formatSize(142, 'fr')).toEqual({ unit: 'mb', n: '142' });
    expect(formatSize(1620, 'fr')).toEqual({ unit: 'gb', n: '1,6' });
    expect(formatSize(1620, 'en')).toEqual({ unit: 'gb', n: '1.6' });
  });
});

const item = (id: string, resolved = false, segment_id: string | null = null): ReviewItem => ({
  id, meeting_id: 'm', segment_id, kind: 'low_confidence', start: 0, end: 1,
  proposed_text: '', alternatives: [], question: '', resolution: resolved ? { action: 'validate' } : null,
});

describe('review', () => {
  const items = [item('a', true), item('b', false, 's2'), item('c', true), item('d', false, 's4')];
  test('send rule: every item resolved', () => {
    expect(remaining(items)).toBe(2);
    expect(canSend(items)).toBe(false);
    expect(canSend([item('a', true)])).toBe(true);
    expect(canSend([])).toBe(true);
  });
  test('j/k stepping clamps', () => {
    expect(stepId(items, null, 1)).toBe('a');
    expect(stepId(items, null, -1)).toBe('d');
    expect(stepId(items, 'b', 1)).toBe('c');
    expect(stepId(items, 'd', 1)).toBe('d');
    expect(stepId(items, 'a', -1)).toBe('a');
    expect(stepId([], null, 1)).toBeNull();
  });
  test('next open item after resolving, wrapping to the top', () => {
    expect(nextOpenAfter(items, 'b')).toBe('d');
    expect(nextOpenAfter(items, 'd')).toBe('b');
    expect(nextOpenAfter([item('x')], 'x')).toBeNull();
  });
  test('flagged segments are the open ones', () => {
    expect([...flaggedSegments(items)]).toEqual(['s2', 's4']);
  });
  test('segment at time', () => {
    const segs = [{ id: '1', start: 0, end: 4 }, { id: '2', start: 4, end: 9 }] as Segment[];
    expect(segmentAt(segs, 4)?.id).toBe('2');
    expect(segmentAt(segs, 12)).toBeNull();
  });
  test('speaker names', () => {
    const t = (k: string, p?: Record<string, string>) => (p ? `${k}:${p.letter}` : k);
    const people = [{ id: 'p1', name: 'Camille', email: null, confirmed: true }];
    expect(speakerName('p1', people, t)).toBe('Camille');
    expect(speakerName('speaker_b', people, t)).toBe('speaker.anon:B');
    expect(speakerName('me', people, t)).toBe('speaker.me');
    expect(speakerName(null, people, t)).toBe('speaker.unknown');
  });
});

test('loopback check for local-mode LLM', () => {
  expect(isLoopbackUrl('http://localhost:11434/v1')).toBe(true);
  expect(isLoopbackUrl('http://127.0.0.1:8080')).toBe(true);
  expect(isLoopbackUrl('http://[::1]:11434')).toBe(true);
  expect(isLoopbackUrl('https://api.openai.com/v1')).toBe(false);
  expect(isLoopbackUrl('http://localhost.evil.com')).toBe(false);
  expect(isLoopbackUrl('pas une url')).toBe(false);
});

describe('missingRequired', () => {
  const m = (id: string, kind: ModelInfo['kind'], installed = false): ModelInfo => ({ id, kind, name: id, size_mb: 1, installed });
  const settings = { dictation: { model: 'whisper-base' }, meeting: { model: 'whisper-small' } } as Settings;
  test('lists the chosen whisper models and one model per required kind', () => {
    const models = [m('whisper-tiny', 'whisper'), m('whisper-base', 'whisper'), m('whisper-small', 'whisper', true),
      m('seg', 'diarization_segmentation'), m('emb', 'diarization_embedding'), m('vad', 'vad')];
    expect(missingRequired(models, settings).map((x) => x.id)).toEqual(['whisper-base', 'seg', 'emb', 'vad']);
  });
  test('is empty once everything needed is installed', () => {
    const models = [m('whisper-base', 'whisper', true), m('whisper-small', 'whisper', true),
      m('seg', 'diarization_segmentation', true), m('emb', 'diarization_embedding', true), m('vad', 'vad', true)];
    expect(missingRequired(models, settings)).toEqual([]);
  });
  test('accepts a parakeet dictation model', () => {
    const s = { dictation: { model: 'parakeet-v3' }, meeting: { model: 'whisper-small' } } as Settings;
    const models = [m('parakeet-v3', 'parakeet'), m('whisper-base', 'whisper'), m('whisper-small', 'whisper', true),
      m('seg', 'diarization_segmentation', true), m('emb', 'diarization_embedding', true), m('vad', 'vad', true)];
    expect(missingRequired(models, s).map((x) => x.id)).toEqual(['parakeet-v3']);
  });
});

describe('addTerm', () => {
  test('appends the trimmed term', () => {
    expect(addTerm(['Barnaby'], '  Barber Concept ')).toEqual(['Barnaby', 'Barber Concept']);
  });
  test('refuses empty terms and case-insensitive duplicates', () => {
    expect(addTerm(['Barnaby'], '   ')).toBeNull();
    expect(addTerm(['Barnaby'], 'barnaby')).toBeNull();
  });
});
