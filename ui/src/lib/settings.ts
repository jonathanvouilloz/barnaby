import type { ProcessingMode, Settings } from './api';

/** Mirrors the backend NetPolicy for local mode: only loopback hosts are allowed. */
export function isLoopbackUrl(url: string): boolean {
  try {
    const host = new URL(url).hostname;
    return host === 'localhost' || host === '[::1]' || /^127(\.\d{1,3}){3}$/.test(host);
  } catch {
    return false;
  }
}

/** Trimmed term appended to the vocabulary, or null when empty or already there (any case). */
export function addTerm(list: string[], raw: string): string[] | null {
  const term = raw.trim();
  if (!term || list.some((x) => x.toLocaleLowerCase() === term.toLocaleLowerCase())) return null;
  return [...list, term];
}

export const OPENROUTER_URL = 'https://openrouter.ai/api/v1';
export const OLLAMA_URL = 'http://localhost:11434/v1';
export const MISTRAL_MODEL = 'mistral-voxtral';

/** Cloud transcription sends audio, so it outranks a cloud summary; otherwise the summary decides. */
export function modeFor(meetingModel: string, summaryCloud: boolean): ProcessingMode {
  if (meetingModel === MISTRAL_MODEL) return 'api';
  return summaryCloud ? 'hybrid' : 'local';
}

/** The summary runs in the cloud: hybrid mode, or api mode with a remote LLM (Mistral alone is api with a local LLM). */
export function cloudSummary(s: Settings): boolean {
  return s.processing_mode === 'hybrid' || (s.processing_mode === 'api' && !isLoopbackUrl(s.llm.base_url));
}
