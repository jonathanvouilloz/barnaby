<script lang="ts">
  import { onMount } from 'svelte';
  import { api, events } from '../lib/api';
  import { app, attempt, go, notify } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatTimestamp } from '../lib/format';

  let now = $state(Date.now());
  let levels = $state({ mic: 0, system: 0 });
  let warnings = $state<string[]>([]);
  let notes = $state<{ at: number; text: string }[]>([]);
  let draft = $state('');
  let stopping = $state(false);

  const meeting = $derived(app.meetings.find((m) => m.id === app.recordingId));
  const elapsed = $derived(meeting ? (now - Date.parse(meeting.started_at)) / 1000 : 0);

  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 500);
    const subs = [
      events.audioLevel((l) => (levels = l)),
      events.captureWarning((key) => {
        const k = key.startsWith('capture.') ? key : `capture.${key}`;
        if (!warnings.includes(k)) warnings.push(k);
      }),
    ];
    return () => {
      clearInterval(timer);
      subs.forEach((p) => p.then((off) => off()));
    };
  });

  async function addNote(e: SubmitEvent) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    draft = '';
    if ((await attempt(() => api.addNote(text))) !== undefined) notes.unshift({ at: elapsed, text });
  }

  async function stop() {
    stopping = true;
    if ((await attempt(api.stopMeeting)) !== undefined) {
      app.recordingId = null;
      notify(t('rec.stopped'));
      go({ name: 'home' });
    }
    stopping = false;
  }

  // Perceived loudness: RMS is small for speech, a square root spreads it over the bar.
  const pct = (v: number) => Math.round(Math.min(1, Math.sqrt(Math.max(0, v))) * 100);
</script>

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <h1>{meeting?.title ?? t('rec.title')}</h1>
    {#if app.recordingId}
      <button class="btn stop" disabled={stopping} onclick={stop}><span class="square" aria-hidden="true"></span>{t('rec.stop')}</button>
    {/if}
  </header>

  <div class="screen-body">
    {#if !app.recordingId}
      <p class="empty">{t('rec.none')}</p>
    {:else}
      <div class="live" role="status">
        <span class="rec-dot" aria-hidden="true"></span>
        <span>{t('rec.live')}</span>
        <span class="sr-only">{t('rec.elapsed')}</span>
        <time class="elapsed tabular" aria-hidden="true">{formatTimestamp(elapsed)}</time>
      </div>

      <div class="warnings" aria-live="polite">
        {#each warnings as w (w)}
          <p class="warning">{t(w)}</p>
        {/each}
      </div>

      <section class="meters" aria-label={t('rec.levels')}>
        {#each [['mic', levels.mic], ['system', levels.system]] as [k, v] (k)}
          <div class="meter">
            <span class="label">{t(`rec.${k}`)}</span>
            <div class="bar" role="meter" aria-label={t(`rec.${k}`)} aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct(v as number)}>
              <div class="fill" style:transform="scaleX({pct(v as number) / 100})"></div>
            </div>
          </div>
        {/each}
      </section>

      <section class="notes" aria-labelledby="notes-title">
        <h2 id="notes-title">{t('rec.notes')}</h2>
        <form onsubmit={addNote}>
          <label class="sr-only" for="note">{t('rec.note_label')}</label>
          <input id="note" type="text" bind:value={draft} placeholder={t('rec.note_placeholder')} autocomplete="off" />
        </form>
        {#if notes.length === 0}
          <p class="secondary">{t('rec.notes_empty')}</p>
        {:else}
          <ul>
            {#each notes as n, i (i)}
              <li><time class="secondary tabular">{formatTimestamp(n.at)}</time><span>{n.text}</span></li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </div>
</section>

<style>
  .screen-body { display: grid; gap: 32px; align-content: start; max-width: 640px; }
  .live { display: flex; align-items: center; gap: 12px; color: var(--text-2); }
  .rec-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--rec); animation: pulse 1.4s ease-in-out infinite; }
  .elapsed { margin-left: auto; font-size: 40px; font-weight: 600; line-height: 1; color: var(--text); letter-spacing: -.01em; }
  @keyframes pulse { 50% { opacity: .35; } }

  .stop { background: var(--rec); border-color: transparent; color: var(--on-rec); }
  .stop:hover:not(:disabled) { background: color-mix(in srgb, var(--rec) 88%, var(--on-rec)); }
  .square { width: 8px; height: 8px; border-radius: 2px; background: currentColor; }

  .warnings { display: grid; gap: 8px; }
  /* Out of the grid flow while empty, but still in the page so the live region keeps announcing. */
  .warnings:empty { position: absolute; }
  .warning {
    padding: 8px 12px; border-radius: var(--r-control);
    background: color-mix(in srgb, var(--warn) 12%, var(--surface));
  }

  .meters { display: grid; gap: 16px; }
  .meter { display: grid; grid-template-columns: 128px 1fr; align-items: center; gap: 16px; }
  .label { font-size: 12.5px; color: var(--text-2); }
  .bar { height: 8px; border-radius: 999px; background: var(--fill-strong); overflow: hidden; }
  .fill {
    height: 100%; border-radius: inherit; background: var(--accent); transform-origin: left;
    transition: transform 100ms linear;
  }

  .notes { display: grid; gap: 12px; }
  ul { display: grid; gap: 4px; }
  li { display: flex; gap: 12px; align-items: baseline; }
  li time { flex: none; }
</style>
