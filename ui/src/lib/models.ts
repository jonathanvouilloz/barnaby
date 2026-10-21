import type { ModelInfo, ModelKind, Settings } from './api';

const ALWAYS_NEEDED: ModelKind[] = ['diarization_segmentation', 'diarization_embedding', 'vad'];

/** Models the app needs and lacks: the dictation and meeting Whisper models chosen in
 *  settings, plus one model of each diarization/VAD kind (the first of the catalog). */
export function missingRequired(models: ModelInfo[], settings: Settings): ModelInfo[] {
  const ids = new Set([settings.dictation.model, settings.meeting.model]);
  for (const kind of ALWAYS_NEEDED) {
    const ofKind = models.filter((m) => m.kind === kind);
    if (ofKind.length && !ofKind.some((m) => m.installed)) ids.add(ofKind[0].id);
  }
  return models.filter((m) => ids.has(m.id) && !m.installed);
}
