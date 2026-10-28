// Contract between the UI and the Rust backend (src-tauri/src/commands.rs).
// Types mirror crates/core/src/model.rs: change both together.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { documentDir, join } from '@tauri-apps/api/path';
import { open } from '@tauri-apps/plugin-dialog';

export type MeetingState =
  | 'recording' | 'captured' | 'queued' | 'processing'
  | 'needs_review' | 'reviewed' | 'sending' | 'sent' | 'failed';
export type ProcessingMode = 'local' | 'api' | 'hybrid';
export type ReviewKind = 'low_confidence' | 'overlap' | 'unattributed_speaker' | 'uncertain_term' | 'inaudible';
export type DictationMode = 'hold' | 'toggle';
export type AudioRetention = 'on_review' | 'seven_days' | 'forever';

export interface Participant { id: string; name: string; email: string | null; confirmed: boolean }

export interface Meeting {
  id: string; title: string; started_at: string; ended_at: string | null;
  language: string | null; state: MeetingState; error: string | null; mode: ProcessingMode;
  review_items_total: number; review_items_resolved: number;
}

/** `meeting-progress` event: `index` is 1-based, `fraction` is the share of the current step done. */
export interface MeetingProgress {
  meeting_id: string; step: 'transcribe' | 'diarize' | 'identify' | 'review' | 'structure';
  index: number; total: number; fraction: number;
}

export interface Segment {
  id: string; start: number; end: number; speaker_id: string | null;
  text: string; confidence: number; reviewed: boolean;
}
export interface Note { id: string; at: number; text: string; source: 'manual' | 'marker' }
export interface ActionItem { owner: string | null; task: string; due: string | null }
export interface Summary { overview: string; decisions: string[]; action_items: ActionItem[]; open_questions: string[] }

export interface MeetingDoc {
  schema_version: string;
  meeting: { id: string; title: string; started_at: string; ended_at: string | null; language: string | null; participants: Participant[] };
  segments: Segment[];
  notes: Note[];
  summary: Summary | null;
  processing: { mode: ProcessingMode; providers: Record<string, string>; review_items_total: number; review_items_resolved: number };
}

export type Resolution =
  | { action: 'validate' }
  | { action: 'correct'; text: string }
  | { action: 'reassign'; speaker_id: string }
  | { action: 'mark_inaudible' }
  | { action: 'ignore' };

export interface ReviewItem {
  id: string; meeting_id: string; segment_id: string | null; kind: ReviewKind;
  start: number; end: number; proposed_text: string; alternatives: string[];
  question: string; resolution: Resolution | null;
}

/** Everything the review screen needs. `audio_path` is null once audio was deleted (retention). */
export interface MeetingDetail { meeting: Meeting; doc: MeetingDoc; review_items: ReviewItem[]; audio_path: string | null }

export interface Dictation {
  id: string; created_at: string; text: string; language: string | null;
  duration_ms: number; injected_into: string | null; provider: string;
}

export interface Settings {
  ui_language: 'fr' | 'en';
  transcription_language: 'auto' | 'fr' | 'en';
  processing_mode: ProcessingMode;
  dictation: { mode: DictationMode; model: string; paste_delay_before_ms: number; keep_history: boolean; show_overlay: boolean; sound_cues: boolean; show_copied: boolean };
  meeting: { model: string; confidence_threshold: number; retention: AudioRetention; excluded_outputs: string[] };
  shortcuts: { dictation: string; meeting_toggle: string; marker: string; review_queue: string };
  destinations: { folder: { enabled: boolean; path: string }; webhook: { enabled: boolean; url: string } };
  llm: { enabled: boolean; base_url: string; model: string };
  consent_acknowledged: boolean;
  notification_sound: boolean;
  /** Terms (brand names, jargon) the transcripts must spell right; also given to the LLM. */
  vocabulary: string[];
  /** The user saw what leaves the machine in hybrid/api mode (spec 6). */
  cloud_disclosure_acknowledged: boolean;
  onboarding_done: boolean;
}

/** 'parakeet': fast dictation model (sherpa-onnx); 'whisper': meetings, dictation fallback. */
export type ModelKind = 'whisper' | 'parakeet' | 'diarization_segmentation' | 'diarization_embedding' | 'vad';
/** A sound output (speakers, headset…) the meeting capture can record. */
export interface OutputDevice { id: string; name: string }
export interface ModelInfo { id: string; kind: ModelKind; name: string; size_mb: number; installed: boolean }

export type DictationStatus = 'idle' | 'recording' | 'transcribing' | 'copied';

export const api = {
  getSettings: () => invoke<Settings>('get_settings'),
  /** Re-registers shortcuts; rejects with a message if one conflicts. */
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  /** Stored in the OS keyring. Empty string clears it. */
  setWebhookSecret: (secret: string) => invoke<void>('set_webhook_secret', { secret }),
  hasWebhookSecret: () => invoke<boolean>('has_webhook_secret'),
  testWebhook: (url: string) => invoke<void>('test_webhook', { url }),
  generateWebhookSecret: () => invoke<string>('generate_webhook_secret'),
  suspendShortcuts: () => invoke<void>('suspend_shortcuts'),
  resumeShortcuts: () => invoke<void>('resume_shortcuts'),
  checkShortcut: (accel: string) => invoke<boolean>('check_shortcut', { accel }),
  /** System folder picker; `null` when cancelled. */
  pickFolder: (defaultPath?: string) => open({ directory: true, defaultPath: defaultPath || undefined }),
  defaultFolder: async () => join(await documentDir(), 'Barnaby'),
  /** Cloud LLM key (OpenRouter), stored in the OS keyring. Empty string clears it. */
  setLlmApiKey: (key: string) => invoke<void>('set_llm_api_key', { key }),
  hasLlmApiKey: () => invoke<boolean>('has_llm_api_key'),
  setMistralApiKey: (key: string) => invoke<void>('set_mistral_api_key', { key }),
  hasMistralApiKey: () => invoke<boolean>('has_mistral_api_key'),

  listMeetings: () => invoke<Meeting[]>('list_meetings'),
  getMeeting: (id: string) => invoke<MeetingDetail>('get_meeting', { id }),
  startMeeting: (title: string, participants: Participant[]) => invoke<string>('start_meeting', { title, participants }),
  stopMeeting: () => invoke<void>('stop_meeting'),
  /** Id of the meeting being recorded, if any. */
  currentRecording: () => invoke<string | null>('current_recording'),
  addNote: (text: string) => invoke<void>('add_note', { text }),
  resolveReviewItem: (id: string, resolution: Resolution) => invoke<void>('resolve_review_item', { id, resolution }),
  /** Renames an anonymous speaker everywhere in the meeting (and remembers the correction). */
  renameSpeaker: (meetingId: string, speakerId: string, name: string) =>
    invoke<void>('rename_speaker', { meetingId, speakerId, name }),
  updateSegmentText: (meetingId: string, segmentId: string, text: string) =>
    invoke<void>('update_segment_text', { meetingId, segmentId, text }),
  /** Marks reviewed and sends to the configured destinations right away. */
  validateReview: (id: string) => invoke<void>('validate_review', { id }),
  retryMeeting: (id: string) => invoke<void>('retry_meeting', { id }),
  deleteMeeting: (id: string) => invoke<void>('delete_meeting', { id }),

  listDictations: () => invoke<Dictation[]>('list_dictations'),
  deleteDictation: (id: string) => invoke<void>('delete_dictation', { id }),
  clearDictations: () => invoke<void>('clear_dictations'),

  listModels: () => invoke<ModelInfo[]>('list_models'),
  listOutputs: () => invoke<OutputDevice[]>('list_outputs'),
  downloadModel: (id: string) => invoke<void>('download_model', { id }),
  deleteModel: (id: string) => invoke<void>('delete_model', { id }),
  startMicTest: () => invoke<void>('start_mic_test'),
  stopMicTest: () => invoke<void>('stop_mic_test'),
};

export const events = {
  meetingUpdated: (cb: (m: Meeting) => void): Promise<UnlistenFn> => listen<Meeting>('meeting-updated', (e) => cb(e.payload)),
  dictationStatus: (cb: (s: DictationStatus) => void) => listen<DictationStatus>('dictation-status', (e) => cb(e.payload)),
  /** Mic RMS in [0, 1], ~20 per second while dictating (overlay waveform). */
  dictationLevel: (cb: (level: number) => void) => listen<number>('dictation-level', (e) => cb(e.payload)),
  /** Mic level in [0, 1], ~20 per second during the onboarding mic test (nothing is recorded). */
  micLevel: (cb: (level: number) => void) => listen<number>('mic-level', (e) => cb(e.payload)),
  dictationDone: (cb: (d: Dictation) => void) => listen<Dictation>('dictation-done', (e) => cb(e.payload)),
  /** RMS levels in [0, 1], ~10 per second while recording a meeting. */
  audioLevel: (cb: (l: { mic: number; system: number }) => void) => listen<{ mic: number; system: number }>('audio-level', (e) => cb(e.payload)),
  /** Shown once: system track silent for 30 s, permission denied, no device... (i18n key). */
  captureWarning: (cb: (key: string) => void) => listen<string>('capture-warning', (e) => cb(e.payload)),
  modelProgress: (cb: (p: { id: string; progress: number }) => void) => listen<{ id: string; progress: number }>('model-progress', (e) => cb(e.payload)),
  /** Backend asks the UI to show a screen (e.g. review queue shortcut). */
  navigate: (cb: (route: string) => void) => listen<string>('navigate', (e) => cb(e.payload)),
};
