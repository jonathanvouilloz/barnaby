import { api, events, type Dictation, type DictationStatus, type Meeting, type Settings } from './api';
import { bindLocale, has, t, type Lang } from './i18n';

export type Route =
  | { name: 'home' }
  | { name: 'new' }
  | { name: 'recording' }
  | { name: 'queue' }
  | { name: 'review'; id: string }
  | { name: 'settings' };

export type OwlState = 'idle' | 'listening' | 'thinking' | 'review' | 'success' | 'error';

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
  celebrate: false,
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

/** Backend error codes map to `error.<code>` (a `:detail` suffix, e.g. `send_failed:webhook`, is dropped); unknown codes are shown as is. */
export function errorText(code: string | null): string {
  const key = `error.${code?.split(':')[0]}`;
  return code && has(key) ? t(key) : (code ?? '');
}

export const reviewCount = () => app.meetings.filter((m) => m.state === 'needs_review').length;

export function owlState(): OwlState {
  if (app.recordingId || app.dictationStatus === 'recording') return 'listening';
  if (app.celebrate) return 'success';
  if (app.dictationStatus === 'transcribing' || app.meetings.some((m) => ['captured', 'queued', 'processing', 'sending'].includes(m.state)))
    return 'thinking';
  if (app.meetings.some((m) => m.state === 'failed')) return 'error';
  if (reviewCount() > 0) return 'review';
  return 'idle';
}

export async function refreshMeetings() {
  const list = await attempt(api.listMeetings);
  if (list) app.meetings = list;
}

function onMeeting(m: Meeting) {
  const i = app.meetings.findIndex((x) => x.id === m.id);
  const previous = i === -1 ? undefined : app.meetings[i].state;
  if (i === -1) app.meetings.unshift(m);
  else app.meetings[i] = m;

  if (m.state === 'recording') {
    app.recordingId = m.id;
    go({ name: 'recording' });
  } else if (m.id === app.recordingId) {
    app.recordingId = null;
    if (app.route.name === 'recording') go({ name: 'home' });
  }
  if (m.state === 'sent' && previous !== 'sent') {
    app.celebrate = true;
    setTimeout(() => (app.celebrate = false), 2400);
  }
}

export async function init() {
  const [settings, meetings, dictations, recording] = await Promise.all([
    attempt(api.getSettings),
    attempt(api.listMeetings),
    attempt(api.listDictations),
    attempt(api.currentRecording),
  ]);
  if (settings) applySettings(settings);
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
    events.navigate((route) => {
      if (route === 'review') go({ name: 'queue' });
      else if (route === 'home' || route === 'settings' || route === 'new') go({ name: route });
    }),
  ]);
}
