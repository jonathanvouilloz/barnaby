<script lang="ts">
  import { api, type Meeting } from '../lib/api';
  import { app, attempt, errorText, go, notify, refreshMeetings } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatDateTime, formatTime } from '../lib/format';
  import StateBadge from '../lib/StateBadge.svelte';
  import Sheet from '../lib/Sheet.svelte';

  let confirmClear = $state(false);
  let toDelete = $state<Meeting | null>(null);

  const shortcuts = $derived(app.settings!.shortcuts);

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
      <button class="btn primary" onclick={() => go({ name: 'new' })}>{t('home.start')}</button>
    {/if}
  </header>

  <div class="screen-body">
    <section class="block" aria-labelledby="meetings-title">
      <h2 id="meetings-title">{t('home.meetings')}</h2>
      {#if app.meetings.length === 0}
        <p class="card empty">{t('home.meetings_empty', { shortcut: shortcuts.meeting_toggle })}</p>
      {:else}
        <ul class="card list">
          {#each app.meetings as m (m.id)}
            {@const clickable = m.state === 'needs_review' || m.state === 'recording'}
            <li class="row">
              <div class="main">
                {#if clickable}
                  <button class="title link" onclick={() => open(m)}>{m.title}</button>
                {:else}
                  <span class="title">{m.title}</span>
                {/if}
                <p class="secondary">
                  {formatDateTime(m.started_at, app.lang)}
                  {#if m.review_items_total > 0}{' · '}{t('home.zones', { done: m.review_items_resolved, total: m.review_items_total })}{/if}
                </p>
                {#if m.state === 'failed'}<p class="cause">{errorText(m.error)}</p>{/if}
              </div>
              <StateBadge state={m.state} />
              {#if m.state === 'failed'}
                <button class="btn small" onclick={() => attempt(() => api.retryMeeting(m.id)).then(refreshMeetings)}>{t('common.retry')}</button>
              {/if}
              {#if m.state !== 'recording'}
                <button class="btn ghost icon" aria-label={t('home.delete_meeting', { title: m.title })} title={t('common.delete')} onclick={() => (toDelete = m)}>
                  {@render trash()}
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="block" aria-labelledby="dictations-title">
      <div class="block-head">
        <h2 id="dictations-title">{t('home.dictations')}</h2>
        {#if app.dictations.length > 0}
          <button class="btn ghost small danger" onclick={() => (confirmClear = true)}>{t('home.clear_all')}</button>
        {/if}
      </div>
      {#if app.dictations.length === 0}
        <p class="card empty">{t('home.dictations_empty', { shortcut: shortcuts.dictation })}</p>
      {:else}
        <ul class="card list">
          {#each app.dictations as d (d.id)}
            {@const time = formatDateTime(d.created_at, app.lang)}
            <li class="row">
              <div class="main">
                <p class="text">{d.text}</p>
                <p class="secondary tabular"><time datetime={d.created_at}>{time}</time>{#if d.injected_into}{` · ${d.injected_into}`}{/if}</p>
              </div>
              <button class="btn small" aria-label={t('home.copy_dictation', { time: formatTime(d.created_at, app.lang) })} onclick={() => copy(d.text)}>{t('common.copy')}</button>
              <button class="btn ghost icon" aria-label={t('home.delete_dictation', { time: formatTime(d.created_at, app.lang) })} title={t('common.delete')} onclick={() => removeDictation(d.id)}>
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
  .block { margin-bottom: 28px; max-width: 760px; }
  .block > h2, .block-head { margin-bottom: 10px; }
  .block-head { display: flex; align-items: center; justify-content: space-between; }
  .list > li + li { border-top: 1px solid var(--border); }
  .row { display: flex; align-items: center; gap: 10px; padding: 10px 10px 10px 16px; }
  .main { flex: 1; min-width: 0; }
  .title { font-weight: 500; }
  .link {
    padding: 0; border: 0; background: none; cursor: pointer; text-align: left;
    color: var(--text); text-decoration: underline; text-decoration-color: transparent; text-underline-offset: 3px;
  }
  .link:hover { text-decoration-color: currentColor; }
  .cause { font-size: 12.5px; color: var(--danger); }
  .text { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
</style>
