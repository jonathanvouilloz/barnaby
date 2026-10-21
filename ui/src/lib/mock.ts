// Fake backend for `bun run dev` in a plain browser. Loaded only when Tauri is absent (see main.ts).
import { mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import type {
  Dictation, Meeting, MeetingDetail, ModelInfo, Participant, Resolution, ReviewItem, Segment, Settings,
} from './api';

const clone = <T>(x: T): T => structuredClone(x);
const today = (h: number, m: number, daysAgo = 0) => {
  const d = new Date();
  d.setDate(d.getDate() - daysAgo);
  d.setHours(h, m, 0, 0);
  return d.toISOString();
};

let settings: Settings = {
  ui_language: 'fr',
  transcription_language: 'auto',
  processing_mode: 'local',
  dictation: { mode: 'toggle', model: 'parakeet-v3', paste_delay_before_ms: 60, paste_delay_after_ms: 300, keep_history: true, show_overlay: true },
  meeting: { model: 'whisper-small', confidence_threshold: 0.6, retention: 'on_review' },
  shortcuts: { dictation: 'Ctrl+Alt+Space', meeting_toggle: 'Ctrl+Shift+R', marker: 'Ctrl+Shift+M', review_queue: 'Ctrl+Shift+L' },
  destinations: { folder: { enabled: true, path: '/home/camille/Documents/Réunions' }, webhook: { enabled: false, url: '' } },
  llm: { enabled: false, base_url: 'http://localhost:11434/v1', model: 'qwen2.5:7b' },
  consent_acknowledged: false,
  notification_sound: false,
  vocabulary: ['Barber Concept', 'Barnaby'],
  cloud_disclosure_acknowledged: false,
};
let webhookSecret = '';
let llmApiKey = '';

const team: Participant[] = [
  { id: 'p_camille', name: 'Camille Martin', email: 'camille.martin@exemple.fr', confirmed: true },
  { id: 'p_julien', name: 'Julien Roux', email: 'julien.roux@exemple.fr', confirmed: true },
];

const seg = (id: string, start: number, end: number, speaker_id: string | null, text: string, confidence = 0.92): Segment =>
  ({ id, start, end, speaker_id, text, confidence, reviewed: false });

const hebdoSegments: Segment[] = [
  seg('s1', 0, 7.5, 'me', 'Bonjour à tous, on fait le point sur la sortie de la version 2.3 et sur le planning de novembre.'),
  seg('s2', 7.5, 16, 'speaker_a', 'Côté design, les maquettes de l’onboarding sont prêtes. Il reste à valider le parcours d’inscription sur mobile.'),
  seg('s3', 16, 24.2, 'speaker_b', 'Pour le back, la migration vers Postgres 17 est faite en recette. On attend le feu vert pour la prod.'),
  seg('s4', 24.2, 31, 'me', 'Très bien. Est-ce qu’on a mesuré l’impact sur les temps de réponse ?'),
  seg('s5', 31, 40.5, 'speaker_b', 'Oui, on est autour de 120 millisecondes au 95e centile, contre 180 avant.', 0.48),
  seg('s6', 40.5, 46, 'speaker_a', 'Attends, c’est mesuré sur quel jeu de données ?', 0.71),
  seg('s7', 46, 55, 'speaker_b', 'Sur la copie anonymisée de la base de septembre.'),
  seg('s8', 55, 63, null, 'Il faudra aussi prévenir le support avant la mise en prod.', 0.83),
  seg('s9', 63, 72, 'me', 'D’accord. Camille, tu peux préparer une note pour l’équipe Zendesk ?', 0.64),
  seg('s10', 72, 80, 'speaker_a', 'Oui, je l’envoie d’ici jeudi.'),
  seg('s11', 80, 90, 'me', 'Parfait. Dernier point : la démo client est décalée au 4 novembre.'),
  seg('s12', 90, 98, 'speaker_b', 'Ça nous laisse le temps de corriger les derniers retours de la recette.'),
];

const item = (id: string, meeting_id: string, s: Segment, kind: ReviewItem['kind'], proposed_text: string, alternatives: string[] = []): ReviewItem =>
  ({ id, meeting_id, segment_id: s.id, kind, start: s.start, end: s.end, proposed_text, alternatives, question: `review.q.${kind}`, resolution: null });

const meeting = (id: string, title: string, started_at: string, minutes: number, state: Meeting['state'], total = 0, resolved = 0, error: string | null = null): Meeting => ({
  id, title, started_at, ended_at: new Date(Date.parse(started_at) + minutes * 60000).toISOString(),
  language: 'fr', state, error, mode: 'local', review_items_total: total, review_items_resolved: resolved,
});

const details = new Map<string, MeetingDetail>();
function addDetail(m: Meeting, participants: Participant[], segments: Segment[], items: ReviewItem[], audio: boolean, summary: MeetingDetail['doc']['summary'] = null) {
  details.set(m.id, {
    meeting: m,
    doc: {
      schema_version: '1.0',
      meeting: { id: m.id, title: m.title, started_at: m.started_at, ended_at: m.ended_at, language: m.language, participants },
      segments,
      notes: [{ id: 'n1', at: 82, text: 'Démo décalée au 4/11', source: 'manual' }],
      summary,
      processing: { mode: 'local', providers: { stt: 'whisper-small', diarization: 'sherpa-onnx' }, review_items_total: items.length, review_items_resolved: 0 },
    },
    review_items: items,
    audio_path: audio ? `/home/camille/.local/share/barnaby/meetings/${m.id}/mix.wav` : null,
  });
}

const hebdo = meeting('m1', 'Point hebdo produit', today(9, 30), 2, 'needs_review', 4, 0);
addDetail(hebdo, team, hebdoSegments, [
  item('r1', 'm1', hebdoSegments[4], 'low_confidence', 'Oui, on est autour de 120 millisecondes au 95e centile, contre 180 avant.', [
    'Oui, on est autour de 120 millisecondes au 95e percentile, contre 180 avant.',
  ]),
  item('r2', 'm1', hebdoSegments[5], 'overlap', 'Attends, c’est mesuré sur quel jeu de données ?'),
  item('r3', 'm1', hebdoSegments[7], 'unattributed_speaker', 'Il faudra aussi prévenir le support avant la mise en prod.'),
  item('r4', 'm1', hebdoSegments[8], 'uncertain_term', 'Zendesk', ['Zen Desk', 'Sendesk']),
], true, {
  overview: 'Point d’avancement sur la version 2.3 : design prêt, migration Postgres validée en recette, démo client décalée au 4 novembre.',
  decisions: ['La démo client est décalée au 4 novembre.'],
  action_items: [
    { owner: 'Camille Martin', task: 'Préparer une note pour l’équipe support', due: 'jeudi' },
    { owner: null, task: 'Valider le parcours d’inscription sur mobile', due: null },
  ],
  open_questions: ['Date du passage en production de la migration Postgres'],
});

const atelierSegs = [
  seg('a1', 0, 9, 'me', 'On commence par les priorités du premier trimestre.'),
  seg('a2', 9, 18, 'speaker_a', 'Pour moi, l’export comptable passe avant le tableau de bord.'),
  seg('a3', 18, 23, 'speaker_b', '', 0.2),
  seg('a4', 23, 31, 'me', 'Noté, on en reparle avec Sophie la semaine prochaine.'),
];
const atelier = meeting('m2', 'Atelier roadmap T1', today(16, 0, 1), 1, 'needs_review', 1, 0);
addDetail(atelier, [{ id: 'p_sophie', name: 'Sophie Lambert', email: null, confirmed: true }], atelierSegs, [
  item('r5', 'm2', atelierSegs[2], 'inaudible', ''),
], false);

const meetings: Meeting[] = [
  hebdo,
  meeting('m5', 'Rétro sprint 42', today(11, 0), 35, 'processing'),
  atelier,
  meeting('m3', 'Entretien fournisseur', today(14, 15, 2), 25, 'failed', 0, 0, 'model_missing'),
  meeting('m4', 'Comité de pilotage', today(10, 0, 4), 50, 'sent', 3, 3),
];

let dictations: Dictation[] = [
  { id: 'd1', created_at: today(10, 42), text: 'Merci pour ton retour, je regarde ça cet après-midi et je te tiens au courant.', language: 'fr', duration_ms: 4200, injected_into: 'Slack', provider: 'whisper-base' },
  { id: 'd2', created_at: today(10, 5), text: 'Ajouter un test pour la reprise après coupure réseau dans le module de synchronisation.', language: 'fr', duration_ms: 5100, injected_into: 'Code', provider: 'whisper-base' },
  { id: 'd3', created_at: today(17, 48, 1), text: 'Bonjour Madame Leroy, je vous confirme notre rendez-vous de mardi à 14 heures dans nos locaux.', language: 'fr', duration_ms: 6300, injected_into: null, provider: 'whisper-base' },
];

const models: ModelInfo[] = [
  { id: 'parakeet-v3', kind: 'parakeet', name: 'Parakeet v3 (dictée rapide)', size_mb: 487, installed: false },
  { id: 'whisper-tiny', kind: 'whisper', name: 'Whisper tiny', size_mb: 78, installed: false },
  { id: 'whisper-base', kind: 'whisper', name: 'Whisper base', size_mb: 148, installed: true },
  { id: 'whisper-small', kind: 'whisper', name: 'Whisper small', size_mb: 488, installed: true },
  { id: 'whisper-large-v3-turbo', kind: 'whisper', name: 'Whisper large-v3 turbo', size_mb: 1625, installed: false },
  { id: 'pyannote-segmentation-3-0', kind: 'diarization_segmentation', name: 'Pyannote segmentation 3.0', size_mb: 7, installed: true },
  // Missing on purpose: the first-run sheet shows up in dev.
  { id: 'wespeaker-voxceleb-resnet34-lm', kind: 'diarization_embedding', name: 'WeSpeaker ResNet34 (VoxCeleb)', size_mb: 27, installed: false },
  { id: 'silero-vad', kind: 'vad', name: 'Silero VAD', size_mb: 1, installed: false },
];

let recording: string | null = null;
let levelTimer: ReturnType<typeof setInterval> | undefined;

const send = (m: Meeting) => emit('meeting-updated', clone(m));

function startRecording(title: string, participants: Participant[]): string {
  const m = meeting(`m${Date.now()}`, title, new Date().toISOString(), 0, 'recording');
  m.ended_at = null;
  meetings.unshift(m);
  addDetail(m, participants, [], [], true);
  recording = m.id;
  let tick = 0;
  levelTimer = setInterval(() => {
    tick++;
    const system = tick > 60 ? 0 : 0.25 + 0.2 * Math.abs(Math.sin(tick / 3));
    emit('audio-level', { mic: 0.15 + 0.35 * Math.random(), system });
    if (tick === 60) emit('capture-warning', 'capture.system_silent');
  }, 100);
  send(m);
  return m.id;
}

function stopRecording() {
  const m = meetings.find((x) => x.id === recording);
  clearInterval(levelTimer);
  recording = null;
  if (!m) return;
  m.state = 'processing';
  m.ended_at = new Date().toISOString();
  send(m);
}

function resolve(id: string, r: Resolution) {
  for (const d of details.values()) {
    const it = d.review_items.find((x) => x.id === id);
    if (!it) continue;
    const s = d.doc.segments.find((x) => x.id === it.segment_id);
    if (s && r.action === 'correct') s.text = it.kind === 'uncertain_term' ? s.text.replace(it.proposed_text, r.text) : r.text;
    if (s && r.action === 'reassign') s.speaker_id = r.speaker_id;
    if (s) s.reviewed = true;
    it.resolution = r;
    d.meeting.review_items_resolved = d.review_items.filter((x) => x.resolution).length;
    const m = meetings.find((x) => x.id === d.meeting.id);
    if (m) {
      m.review_items_resolved = d.meeting.review_items_resolved;
      send(m);
    }
    return;
  }
  throw 'unknown review item';
}

function setState(id: string, state: Meeting['state']) {
  const m = meetings.find((x) => x.id === id);
  if (!m) throw 'unknown meeting';
  m.state = state;
  if (state !== 'failed') m.error = null;
  send(m);
}

function handle(cmd: string, a: Record<string, any>): unknown {
  switch (cmd) {
    case 'get_settings': return clone(settings);
    case 'save_settings': {
      const s = a.settings as Settings;
      const values = Object.values(s.shortcuts);
      const dup = values.find((v, i) => values.indexOf(v) !== i);
      if (dup) throw `Le raccourci ${dup} est utilisé deux fois.`;
      if (values.includes('Ctrl+Alt+Delete')) throw 'Ctrl+Alt+Delete est réservé par le système.';
      settings = clone(s);
      return null;
    }
    case 'set_webhook_secret': webhookSecret = a.secret; return null;
    case 'has_webhook_secret': return webhookSecret !== '';
    case 'set_llm_api_key': llmApiKey = a.key; return null;
    case 'has_llm_api_key': return llmApiKey !== '';
    case 'list_meetings': return clone(meetings);
    case 'get_meeting': {
      const d = details.get(a.id);
      if (!d) throw 'unknown meeting';
      d.meeting = clone(meetings.find((m) => m.id === a.id)!);
      return clone(d);
    }
    case 'start_meeting': return startRecording(a.title, a.participants);
    case 'stop_meeting': stopRecording(); return null;
    case 'current_recording': return recording;
    case 'add_note': return null;
    case 'resolve_review_item': resolve(a.id, a.resolution); return null;
    case 'rename_speaker': {
      const d = details.get(a.meetingId)!;
      const ps = d.doc.meeting.participants;
      let p = ps.find((x) => x.name === a.name);
      if (!p) ps.push((p = { id: `p_${Date.now()}`, name: a.name, email: null, confirmed: true }));
      for (const s of d.doc.segments) if (s.speaker_id === a.speakerId) s.speaker_id = p.id;
      return null;
    }
    case 'update_segment_text': {
      const s = details.get(a.meetingId)?.doc.segments.find((x) => x.id === a.segmentId);
      if (!s) throw 'unknown segment';
      s.text = a.text;
      return null;
    }
    case 'validate_review':
      setState(a.id, 'sending');
      setTimeout(() => setState(a.id, 'sent'), 1200);
      return null;
    case 'retry_meeting':
      setState(a.id, 'queued');
      setTimeout(() => setState(a.id, 'processing'), 800);
      return null;
    case 'delete_meeting': {
      const i = meetings.findIndex((m) => m.id === a.id);
      if (i !== -1) meetings.splice(i, 1);
      details.delete(a.id);
      return null;
    }
    case 'list_dictations': return clone(dictations);
    case 'delete_dictation': dictations = dictations.filter((d) => d.id !== a.id); return null;
    case 'clear_dictations': dictations = []; return null;
    case 'list_models': return clone(models);
    case 'download_model': {
      const m = models.find((x) => x.id === a.id)!;
      let p = 0;
      const timer = setInterval(() => {
        p = Math.min(1, p + 0.07);
        if (p >= 1) {
          clearInterval(timer);
          m.installed = true;
        }
        emit('model-progress', { id: m.id, progress: p });
      }, 150);
      return null;
    }
    case 'delete_model': models.find((x) => x.id === a.id)!.installed = false; return null;
  }
  throw `mock: unknown command ${cmd}`;
}

/** One silent mono WAV so the review player can seek and play in the browser. */
function silentWav(seconds: number): string {
  const rate = 8000, n = rate * seconds;
  const buf = new DataView(new ArrayBuffer(44 + n));
  const str = (o: number, s: string) => [...s].forEach((c, i) => buf.setUint8(o + i, c.charCodeAt(0)));
  str(0, 'RIFF'); buf.setUint32(4, 36 + n, true); str(8, 'WAVEfmt ');
  buf.setUint32(16, 16, true); buf.setUint16(20, 1, true); buf.setUint16(22, 1, true);
  buf.setUint32(24, rate, true); buf.setUint32(28, rate, true); buf.setUint16(32, 1, true); buf.setUint16(34, 8, true);
  str(36, 'data'); buf.setUint32(40, n, true);
  for (let i = 0; i < n; i++) buf.setUint8(44 + i, 128);
  return URL.createObjectURL(new Blob([buf], { type: 'audio/wav' }));
}

export function installMock() {
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, any>), { shouldMockEvents: true });
  const wav = silentWav(100);
  (window as any).__TAURI_INTERNALS__.convertFileSrc = () => wav;
  // Lets a test driver (Playwright) simulate backend events.
  (window as any).__barnabyMock = { emit };
  // Overlay window: fake mic levels, bursts of speech with short pauses.
  if (location.hash === '#overlay') {
    let tick = 0;
    setInterval(() => {
      tick++;
      const speaking = tick % 40 < 28;
      emit('dictation-level', speaking ? 0.03 + 0.12 * Math.abs(Math.sin(tick / 2.3)) * Math.random() + 0.04 * Math.random() : 0.004);
    }, 50);
  }
}
