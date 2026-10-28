import type { ModelInfo, Settings } from '../lib/api';
import { missingRequired } from '../lib/models';
import { MISTRAL_MODEL } from '../lib/settings';

export type Step = 'welcome' | 'mic' | 'models' | 'destination' | 'dictation' | 'tour' | 'done';
export const STEPS: Step[] = ['welcome', 'mic', 'models', 'destination', 'dictation', 'tour', 'done'];

/** Upgrades that already have every model needed are marked done without being shown the flow. */
export function shouldOnboard(settings: Settings, missing: ModelInfo[]): 'show' | 'skip' | 'mark_done' {
  if (settings.onboarding_done) return 'skip';
  return missing.length ? 'show' : 'mark_done';
}

/** Meeting model saved for a choice: local keeps the chosen Whisper, or falls back to the default after Mistral. */
export function meetingModelFor(choice: 'local' | 'cloud', settings: Settings): string {
  if (choice === 'cloud') return MISTRAL_MODEL;
  return settings.meeting.model === MISTRAL_MODEL ? 'whisper-large-v3-turbo' : settings.meeting.model;
}

export function modelsFor(choice: 'local' | 'cloud', models: ModelInfo[], settings: Settings): ModelInfo[] {
  return missingRequired(models, { ...settings, meeting: { ...settings.meeting, model: meetingModelFor(choice, settings) } });
}
