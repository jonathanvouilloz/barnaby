import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { translate, dictionaries } from './i18n';
import { acceleratorFromKey, keyLabel, type KeyLike } from './accelerator';
import { formatTimestamp, formatSize, groupByDay, localDay } from './format';
import { canSend, nextOpenAfter, remaining, segmentAt, speakerName, stepId, flaggedSegments, wordDiff } from './review';
import { addTerm, consentTemplate, isLoopbackUrl, MISTRAL_MODEL, modeFor } from './settings';
import type { ModelInfo, ReviewItem, Segment, Settings } from './api';
import { missingRequired } from './models';
import { modelsFor, shouldOnboard, STEPS } from '../onboarding/steps';

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
  test('displays keycaps in the interface language', () => {
    expect(keyLabel('Ctrl+Alt+Space', 'fr')).toBe('Ctrl + Alt + Espace');
    expect(keyLabel('Ctrl+Shift+R', 'fr')).toBe('Ctrl + Maj + R');
    expect(keyLabel('Ctrl+Shift+Space', 'en')).toBe('Ctrl + Shift + Space');
    expect(keyLabel('F9', 'fr')).toBe('F9');
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

describe('onboarding', () => {
  const m = (id: string, kind: ModelInfo['kind'], installed = false): ModelInfo => ({ id, kind, name: id, size_mb: 1, installed });
  const settings = { dictation: { model: 'parakeet-v3' }, meeting: { model: 'whisper-small' } } as Settings;
  const catalog = [m('parakeet-v3', 'parakeet'), m('whisper-small', 'whisper'), m('whisper-base', 'whisper'),
    m('seg', 'diarization_segmentation'), m('emb', 'diarization_embedding'), m('vad', 'vad')];
  test('steps come in order', () => {
    expect(STEPS).toEqual(['welcome', 'mic', 'models', 'dictation', 'tour', 'done']);
  });
  test('shouldOnboard shows, marks done or skips', () => {
    expect(shouldOnboard({ ...settings, onboarding_done: false }, [catalog[0]])).toBe('show');
    expect(shouldOnboard({ ...settings, onboarding_done: false }, [])).toBe('mark_done');
    expect(shouldOnboard({ ...settings, onboarding_done: true }, [catalog[0]])).toBe('skip');
  });
  test('cloud choice drops the meeting whisper but keeps dictation and diarization', () => {
    expect(modelsFor('cloud', catalog, settings).map((x) => x.id)).toEqual(['parakeet-v3', 'seg', 'emb', 'vad']);
  });
  test('local choice includes the meeting model, even after a cloud choice', () => {
    expect(modelsFor('local', catalog, settings).map((x) => x.id)).toEqual(['parakeet-v3', 'whisper-small', 'seg', 'emb', 'vad']);
    const cloud = { ...settings, meeting: { model: MISTRAL_MODEL } } as Settings;
    expect(modelsFor('local', catalog, cloud).map((x) => x.id)).toContain('whisper-small');
  });
  test('an installed model is never offered again', () => {
    const resumed = catalog.map((x) => (x.id === 'parakeet-v3' || x.id === 'seg' ? { ...x, installed: true } : x));
    expect(modelsFor('local', resumed, settings).map((x) => x.id)).toEqual(['whisper-small', 'emb', 'vad']);
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

test('processing mode follows meeting model and summary', () => {
  expect(modeFor(MISTRAL_MODEL, false)).toBe('api');
  expect(modeFor(MISTRAL_MODEL, true)).toBe('api');
  expect(modeFor('whisper-small', true)).toBe('hybrid');
  expect(modeFor('whisper-small', false)).toBe('local');
});

describe('consent template', () => {
  const local = { enabled: true, base_url: 'http://localhost:11434/v1' } as Settings['llm'];
  const remote = { enabled: true, base_url: 'https://openrouter.ai/api/v1' } as Settings['llm'];
  const cfg = (model: string, retention: Settings['meeting']['retention'], processing_mode: Settings['processing_mode'] = 'local', llm = local) =>
    ({ meeting: { model, retention } as Settings['meeting'], processing_mode, llm });
  const tpl = (...a: Parameters<typeof cfg>) => consentTemplate('fr', cfg(...a));
  test('local model keeps the audio on the computer', () => {
    expect(tpl('whisper-small', 'on_review')).toContain('reste sur mon ordinateur');
    expect(tpl('whisper-small', 'on_review')).not.toContain('Mistral');
  });
  test('Mistral is named and the audio is not said to stay local or be used only for the notes', () => {
    const s = tpl(MISTRAL_MODEL, 'on_review', 'api');
    expect(s).toContain('Mistral');
    expect(s).toContain('selon ses conditions');
    expect(s).not.toContain('reste sur mon ordinateur');
    expect(s).not.toContain('uniquement');
  });
  test('a cloud summary says the transcript goes online', () => {
    const online = 'pour rédiger le résumé';
    expect(tpl('whisper-small', 'on_review', 'hybrid', remote)).toContain(online);
    expect(tpl(MISTRAL_MODEL, 'on_review', 'api', remote)).toContain(online);
    expect(tpl(MISTRAL_MODEL, 'on_review', 'api', local)).not.toContain(online);
    expect(tpl('whisper-small', 'on_review')).not.toContain(online);
    expect(tpl('whisper-small', 'on_review', 'hybrid', { ...remote, enabled: false })).not.toContain(online);
  });
  test('states the real retention', () => {
    expect(tpl('whisper-small', 'on_review')).toContain('une fois le compte rendu relu');
    expect(tpl('whisper-small', 'seven_days')).toContain('sept jours');
    expect(tpl('whisper-small', 'forever')).toContain('Je le conserve');
    expect(tpl('whisper-small', 'forever')).not.toContain('supprime');
  });
  test('participants are addressed with vous, in English too', () => {
    expect(tpl(MISTRAL_MODEL, 'forever')).toContain('Si vous préférez');
    expect(consentTemplate('en', cfg(MISTRAL_MODEL, 'seven_days', 'api'))).toContain('Mistral');
  });
});

describe('wordDiff', () => {
  test('marks only the differing words', () => {
    expect(wordDiff('contre 180 avant', 'contre 120 avant')).toEqual([
      { text: 'contre ', changed: false }, { text: '120', changed: true }, { text: ' avant', changed: false },
    ]);
  });
  test('identical or fully different texts', () => {
    expect(wordDiff('même texte', 'même texte')).toEqual([{ text: 'même texte', changed: false }]);
    expect(wordDiff('abc', 'xyz')).toEqual([{ text: 'xyz', changed: true }]);
  });
  test('an insertion at the end', () => {
    expect(wordDiff('Postgres', 'Postgres 17')).toEqual([{ text: 'Postgres ', changed: false }, { text: '17', changed: true }]);
  });
});

describe('days', () => {
  test('localDay reads the local calendar day', () => {
    expect(localDay(new Date(2026, 0, 5, 23, 59).toISOString())).toBe('2026-01-05');
    expect(localDay(new Date(2026, 0, 6, 0, 1).toISOString())).toBe('2026-01-06');
  });
  test('groupByDay keeps first-seen day order and item order', () => {
    const at = (d: number, h: number) => ({ at: new Date(2026, 0, d, h).toISOString(), d, h });
    const items = [at(6, 9), at(5, 18), at(6, 8), at(3, 10)];
    const groups = groupByDay(items, (x) => x.at);
    expect(groups.map((g) => g.day)).toEqual(['2026-01-06', '2026-01-05', '2026-01-03']);
    expect(groups[0].items.map((x) => x.h)).toEqual([9, 8]);
    expect(groupByDay([], (x: { at: string }) => x.at)).toEqual([]);
  });
});

describe('design tokens', () => {
  test('no literal colour in .svelte files', () => {
    const root = new URL('..', import.meta.url).pathname;
    const hits: string[] = [];
    for (const file of new Bun.Glob('**/*.svelte').scanSync(root)) {
      const src = readFileSync(root + file, 'utf8');
      for (const m of src.matchAll(/#(?:[0-9a-f]{3}|[0-9a-f]{6})\b|\brgba?\(|\bhsla?\(/gi)) hits.push(`${file}: ${m[0]}`);
    }
    expect(hits).toEqual([]);
  });
});
