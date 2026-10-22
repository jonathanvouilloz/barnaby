// Turns a keydown into a Tauri global-shortcut accelerator string, e.g. `Ctrl+Alt+Space`.
import { dictionaries, type Lang } from './i18n';

export interface KeyLike { key: string; code: string; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }

const MODIFIER_KEYS = new Set(['Control', 'Alt', 'AltGraph', 'Shift', 'Meta', 'OS', 'Super', 'Hyper']);

/**
 * `null` while only modifiers are held (keep listening), `'need_modifier'` for a bare key
 * (a global shortcut without modifier would swallow normal typing), else the accelerator.
 * Uses `code` so the result does not depend on the keyboard layout.
 */
export function acceleratorFromKey(e: KeyLike): string | 'need_modifier' | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const key = keyName(e.code);
  if (!key) return null;
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
  if (mods.length === 0 && !/^F\d{1,2}$/.test(key)) return 'need_modifier';
  return [...mods, key].join('+');
}

function keyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1];
  const digit = /^Digit(\d)$/.exec(code);
  if (digit) return digit[1];
  // Other W3C codes (Space, F5, ArrowUp, Comma, Numpad1...) are accepted as is by global-hotkey.
  return code && code !== 'Unidentified' ? code : null;
}

export const MAC = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform || navigator.userAgent);

/**
 * Accelerator as the keycaps read in the interface language: `Ctrl+Shift+Space` → « Ctrl + Maj + Espace ».
 * On a Mac, `Super` and `Alt` read as the keys printed there: Cmd and Option.
 */
export function keyLabel(accel: string, lang: Lang, mac = MAC): string {
  const d = dictionaries[lang];
  return accel.split('+').map((k) => (mac && d[`key.mac.${k}`]) || d[`key.${k}`] || k).join(' + ');
}
