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
