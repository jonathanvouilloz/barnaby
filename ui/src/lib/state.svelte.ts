import { api, events, type Dictation, type DictationStatus, type Meeting, type ModelInfo, type Settings } from './api';
import { bindLocale, has, t, type Lang } from './i18n';
import { keyLabel } from './accelerator';
import { missingRequired } from './models';

export type Route =
  | { name: 'home' }
  | { name: 'new' }
  | { name: 'recording' }
  | { name: 'queue' }
  | { name: 'review'; id: string }
  | { name: 'settings' }
  | { name: 'onboarding' };

export type OwlState = 'idle' | 'listening' | 'thinking' | 'success' | 'error';

const MASCOT_KEY = 'barnaby.mascot';

function readMascot(): boolean {
  try {
    return localStorage.getItem(MASCOT_KEY) !== 'off';
  } catch {
    return true;
  }
}

export const app = $state({
  route: { name: 'home' } as Route,
  lang: 'fr' as Lang,
  settings: null as Settings | null,
  meetings: [] as Meeting[],
  dictations: [] as Dictation[],
  dictationStatus: 'idle' as DictationStatus,
  recordingId: null as string | null,
  notice: '',
  models: [] as ModelInfo[],
  mascot: readMascot(),
});

bindLocale(app);

export function go(route: Route) {
  app.route = route;
}

export function setMascot(on: boolean) {
  app.mascot = on;
  try {
    localStorage.setItem(MASCOT_KEY, on ? 'on' : 'off');
  } catch {
    // private mode or blocked storage: the choice lasts for this session only
  }
}

let noticeTimer: ReturnType<typeof setTimeout> | undefined;
export function notify(message: string) {
  app.notice = message;
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (app.notice = ''), 5000);
}

/** Runs a backend call and reports a failure in the notice area instead of throwing. */
export async function attempt<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn();
  } catch (e) {
    notify(t('error.generic', { message: String(e) }));
    return undefined;
  }
}

export function applySettings(s: Settings) {
  app.settings = s;
  app.lang = s.ui_language;
  document.documentElement.lang = s.ui_language;
}

/** Backend error codes map to `error.<code>`; unknown codes are shown as is.
 *  The `:detail` suffix is offered as `{keys}` (`shortcut_conflict:Ctrl+D: …` names the refused shortcut). */
export function errorText(code: string | null): string {
  const [name, detail = ''] = code?.split(':') ?? [];
  const key = `error.${name}`;
  return code && has(key) ? t(key, { keys: keyLabel(detail, app.lang) }) : (code ?? '');
}

export const reviewCount = () => app.meetings.filter((m) => m.state === 'needs_review').length;

export async function refreshModels() {
  app.models = (await attempt(api.listModels)) ?? app.models;
}

/** Models the app needs and lacks (often after a skipped onboarding step), marked with a spark until installed. */
export const missingModels = () => (app.settings ? missingRequired(app.models, app.settings) : []);

export async function refreshMeetings() {
  const list = await attempt(api.listMeetings);
  if (list) app.meetings = list;
}

function onMeeting(m: Meeting) {
  const i = app.meetings.findIndex((x) => x.id === m.id);
  if (i === -1) app.meetings.unshift(m);
  else app.meetings[i] = m;

  if (m.state === 'recording') {
    app.recordingId = m.id;
    go({ name: 'recording' });
  } else if (m.id === app.recordingId) {
    app.recordingId = null;
    if (app.route.name === 'recording') go({ name: 'home' });
  }
}

export async function init() {
  const [settings, meetings, dictations, recording, models] = await Promise.all([
    attempt(api.getSettings),
    attempt(api.listMeetings),
    attempt(api.listDictations),
    attempt(api.currentRecording),
    attempt(api.listModels),
  ]);
  if (settings) applySettings(settings);
  app.models = models ?? [];
  app.meetings = meetings ?? [];
  app.dictations = dictations ?? [];
  if (recording) {
    app.recordingId = recording;
    go({ name: 'recording' });
  }

  await Promise.all([
    events.meetingUpdated(onMeeting),
    events.dictationStatus((s) => (app.dictationStatus = s)),
    events.dictationDone((d) => {
      if (app.settings?.dictation.keep_history !== false) app.dictations.unshift(d);
    }),
    // A download may finish on any screen, even after its onboarding step was skipped.
    events.modelProgress(({ progress }) => progress >= 1 && refreshModels()),
    events.navigate((route) => {
      // The tray and shortcuts do not cut the onboarding short; a real recording still takes over (onMeeting).
      if (app.route.name === 'onboarding') return;
      if (route === 'review') go({ name: 'queue' });
      else if (route === 'home' || route === 'settings' || route === 'new') go({ name: route });
    }),
  ]);
}
