<script lang="ts">
  import { api } from '../lib/api';
  import { app, attempt, errorText, go, refreshMeetings } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatDateTime } from '../lib/format';
  import StateBadge from '../lib/StateBadge.svelte';

  const queue = $derived(app.meetings.filter((m) => m.state === 'needs_review' || m.state === 'failed'));
</script>

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <h1>{t('queue.title')}</h1>
  </header>
  <div class="screen-body">
    {#if queue.length === 0}
      <p class="card empty">{t('queue.empty')}</p>
    {:else}
      <ul class="card list">
        {#each queue as m (m.id)}
          <li>
            {#if m.state === 'needs_review'}
              <button class="row open" onclick={() => go({ name: 'review', id: m.id })}>
                <span class="main">
                  <span class="title">{m.title}</span>
                  <span class="secondary">{formatDateTime(m.started_at, app.lang)} · {t('review.progress', { done: m.review_items_resolved, total: m.review_items_total })}</span>
                </span>
                <StateBadge state={m.state} />
                <span class="chevron" aria-hidden="true">›</span>
              </button>
            {:else}
              <div class="row">
                <span class="main">
                  <span class="title">{m.title}</span>
                  <span class="cause">{errorText(m.error)}</span>
                </span>
                <StateBadge state={m.state} />
                <button class="btn small" onclick={() => attempt(() => api.retryMeeting(m.id)).then(refreshMeetings)}>{t('common.retry')}</button>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

<style>
  .list { max-width: 760px; }
  .list > li + li { border-top: 1px solid var(--border); }
  .row { display: flex; align-items: center; gap: 12px; width: 100%; padding: 12px 14px 12px 16px; }
  .open {
    border: 0; background: none; text-align: left; cursor: pointer; border-radius: var(--r-card);
    transition: background-color 150ms var(--ease);
  }
  .open:hover { background: var(--fill); }
  .main { flex: 1; min-width: 0; display: grid; }
  .title { font-weight: 500; }
  .cause { font-size: 12.5px; color: var(--danger); }
  .chevron { color: var(--text-2); font-size: 18px; line-height: 1; }
</style>
