<script lang="ts">
  import { api, type Meeting } from '../lib/api';
  import { app, attempt, errorText, go, notify, refreshMeetings } from '../lib/state.svelte';
  import { keyLabel } from '../lib/accelerator';
  import { t } from '../lib/i18n';
  import { formatDateTime, formatDay, formatTime, groupByDay, localDay } from '../lib/format';
  import Owl from '../lib/Owl.svelte';
  import Spark from '../lib/Spark.svelte';
  import Sheet from '../lib/Sheet.svelte';

  let confirmClear = $state(false);
  let toDelete = $state<Meeting | null>(null);

  const shortcuts = $derived(app.settings!.shortcuts);

  function dayLabel(day: string) {
    const now = new Date();
    if (day === localDay(now.toISOString())) return t('home.today');
    now.setDate(now.getDate() - 1);
    if (day === localDay(now.toISOString())) return t('home.yesterday');
    return formatDay(day, app.lang);
  }

  function open(m: Meeting) {
    if (m.state === 'needs_review') go({ name: 'review', id: m.id });
    else if (m.state === 'recording') go({ name: 'recording' });
  }

  async function copy(text: string) {
    await attempt(() => navigator.clipboard.writeText(text));
    notify(t('common.copied'));
  }

  async function removeDictation(id: string) {
    if ((await attempt(() => api.deleteDictation(id))) !== undefined) app.dictations = app.dictations.filter((d) => d.id !== id);
  }

  async function clearAll() {
    confirmClear = false;
    if ((await attempt(api.clearDictations)) !== undefined) app.dictations = [];
  }

  async function deleteMeeting() {
    const m = toDelete!;
    toDelete = null;
    if ((await attempt(() => api.deleteMeeting(m.id))) !== undefined) app.meetings = app.meetings.filter((x) => x.id !== m.id);
  }
</script>

{#snippet trash()}
  <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" /></svg>
{/snippet}

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <h1>{t('home.title')}</h1>
    {#if app.recordingId}
      <button class="btn primary" onclick={() => go({ name: 'recording' })}>{t('rec.live')}</button>
    {:else}
      <button class="btn primary" onclick={() => go({ name: 'new' })}>{t('new.title')}</button>
    {/if}
  </header>

  <div class="screen-body">
    <section class="block" aria-labelledby="meetings-title">
      <h2 id="meetings-title">{t('home.meetings')}</h2>
      {#if app.meetings.length === 0}
        <div class="empty">
          {#if app.mascot}<Owl state="idle" size={72} />{/if}
          <p>{t('home.meetings_empty', { shortcut: keyLabel(shortcuts.meeting_toggle, app.lang) })}</p>
        </div>
      {:else}
        {#each groupByDay(app.meetings, (m) => m.started_at) as group (group.day)}
          <h3 class="day">{dayLabel(group.day)}</h3>
          <ul>
            {#each group.items as m (m.id)}
              {@const clickable = m.state === 'needs_review' || m.state === 'recording'}
              <li class="row">
                <span class="mark"><Spark active={m.state === 'needs_review'} /></span>
                <div class="main">
                  {#if clickable}
                    <button class="title link" onclick={() => open(m)}>{m.title}</button>
                  {:else}
                    <span class="title">{m.title}</span>
                  {/if}
                  <p class="secondary tabular">
                    {formatTime(m.started_at, app.lang)}
                    {#if m.review_items_total > 0}{' · '}{t('home.zones', { done: m.review_items_resolved, total: m.review_items_total })}{/if}
                  </p>
                  {#if m.state === 'failed'}<p class="cause">{errorText(m.error)}</p>{/if}
                </div>
                <span class="state {m.state}">{t(`state.${m.state}`)}</span>
                {#if m.state === 'failed'}
                  <button class="btn small" onclick={() => attempt(() => api.retryMeeting(m.id)).then(refreshMeetings)}>{t('common.retry')}</button>
                {/if}
                {#if m.state !== 'recording'}
                  <button class="btn ghost icon small reveal" aria-label={t('home.delete_meeting', { title: m.title })} title={t('common.delete')} onclick={() => (toDelete = m)}>
                    {@render trash()}
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/each}
      {/if}
    </section>

    <section class="block dictations" aria-labelledby="dictations-title">
      <div class="block-head">
        <h2 id="dictations-title">{t('home.dictations')}</h2>
        {#if app.dictations.length > 0}
          <button class="btn ghost small danger" onclick={() => (confirmClear = true)}>{t('home.clear_all')}</button>
        {/if}
      </div>
      {#if app.dictations.length === 0}
        <p class="secondary quiet">{t('home.dictations_empty', { shortcut: keyLabel(shortcuts.dictation, app.lang) })}</p>
      {:else}
        <ul>
          {#each app.dictations as d (d.id)}
            {@const time = formatDateTime(d.created_at, app.lang)}
            <li class="row">
              <div class="main">
                <p class="text">{d.text}</p>
                <p class="secondary tabular"><time datetime={d.created_at}>{time}</time>{#if d.injected_into}{` · ${d.injected_into}`}{/if}</p>
              </div>
              <button class="btn ghost small reveal" aria-label={t('home.copy_dictation', { time: formatTime(d.created_at, app.lang) })} onclick={() => copy(d.text)}>{t('common.copy')}</button>
              <button class="btn ghost icon small reveal" aria-label={t('home.delete_dictation', { time: formatTime(d.created_at, app.lang) })} title={t('common.delete')} onclick={() => removeDictation(d.id)}>
                {@render trash()}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</section>

<Sheet open={confirmClear} title={t('home.clear_title')} onclose={() => (confirmClear = false)}>
  <p>{t('home.clear_body')}</p>
  {#snippet actions()}
    <button class="btn" onclick={() => (confirmClear = false)}>{t('common.cancel')}</button>
    <button class="btn danger-fill" onclick={clearAll}>{t('home.clear_all')}</button>
  {/snippet}
</Sheet>

<Sheet open={toDelete !== null} title={t('home.delete_meeting_title')} onclose={() => (toDelete = null)}>
  <p><strong>{toDelete?.title}</strong></p>
  <p>{t('home.delete_meeting_body')}</p>
  {#snippet actions()}
    <button class="btn" onclick={() => (toDelete = null)}>{t('common.cancel')}</button>
    <button class="btn danger-fill" onclick={deleteMeeting}>{t('common.delete')}</button>
  {/snippet}
</Sheet>

<style>
  .block { max-width: 760px; }
  .block + .block { margin-top: 40px; }
  .block > h2, .block-head { margin-bottom: 8px; }
  .block-head { display: flex; align-items: center; justify-content: space-between; min-height: 28px; }

  .day {
    margin: 20px 0 4px; padding: 0 8px 4px 32px;
    font-size: 12.5px; font-weight: 650; color: var(--text-2);
    border-bottom: 1px solid var(--border);
  }
  .day::first-letter { text-transform: uppercase; }

  /* Things-like rows: no borders, a soft fill on hover, quiet actions until the row is touched. */
  .row {
    position: relative; display: flex; align-items: center; gap: 8px;
    min-height: 44px; padding: 6px 8px; border-radius: var(--r-control);
    transition: background-color 150ms var(--ease);
  }
  .row:hover { background: var(--fill); }
  .mark { display: inline-flex; justify-content: center; width: 16px; flex: none; }
  .main { flex: 1; min-width: 0; }
  .title { font-weight: 500; }
  .link {
    padding: 0; border: 0; background: none; cursor: pointer; text-align: left;
    font-size: 14px; color: var(--text);
  }
  /* The whole row opens the meeting; the other buttons sit above this layer. */
  .link::after { content: ''; position: absolute; inset: 0; border-radius: inherit; }
  .link:focus-visible { outline: none; }
  .link:focus-visible::after { outline: 2px solid var(--accent); outline-offset: -2px; border-radius: var(--r-control); }
  .row .btn { position: relative; }

  .state { font-size: 12.5px; color: var(--text-2); white-space: nowrap; }
  .state.needs_review, .state.recording { color: var(--text); font-weight: 550; }
  .state.failed { color: var(--danger); }
  .cause { font-size: 12.5px; color: var(--danger); }

  .reveal { opacity: 0; transition: opacity 150ms var(--ease); }
  .row:hover .reveal, .row:focus-within .reveal { opacity: 1; }
  @media (hover: none) { .reveal { opacity: 1; } }

  .text { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .dictations .row { padding-left: 32px; }
  .quiet { padding: 4px 8px; }

  .empty { display: grid; justify-items: center; gap: 12px; padding: 40px 16px; }
  .empty p { max-width: 40ch; }
</style>
