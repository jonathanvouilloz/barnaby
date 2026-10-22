<script lang="ts">
  import { api } from '../lib/api';
  import { app, attempt, errorText, go, refreshMeetings } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatDateTime } from '../lib/format';
  import Icon from '../lib/Icon.svelte';
  import Owl from '../lib/Owl.svelte';
  import Spark from '../lib/Spark.svelte';

  const queue = $derived(app.meetings.filter((m) => m.state === 'needs_review' || m.state === 'failed'));
</script>

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <h1>{t('queue.title')}</h1>
  </header>
  <div class="screen-body">
    {#if queue.length === 0}
      <div class="empty">
        {#if app.mascot}<Owl state="success" size={72} />{/if}
        <p>{t('queue.empty')}</p>
      </div>
    {:else}
      <ul class="list">
        {#each queue as m (m.id)}
          {@const left = m.review_items_total - m.review_items_resolved}
          <li>
            {#if m.state === 'needs_review'}
              <button class="row open" onclick={() => go({ name: 'review', id: m.id })}>
                <span class="mark"><Spark /></span>
                <span class="main">
                  <span class="title">{m.title}</span>
                  <span class="secondary tabular">{formatDateTime(m.started_at, app.lang)} · {t('review.progress', { done: m.review_items_resolved, total: m.review_items_total })}</span>
                </span>
                {#if left > 0}<span class="count tabular" aria-label={t('nav.review_badge', { n: left })}><Spark size={10} />{left}</span>{/if}
                <span class="chevron"><Icon name="chevron" /></span>
              </button>
            {:else}
              <div class="row">
                <span class="mark"></span>
                <span class="main">
                  <span class="title">{m.title}</span>
                  <span class="cause">{#if app.mascot}<Owl state="error" size={32} />{/if}{errorText(m.error)}</span>
                </span>
                <span class="state">{t(`state.${m.state}`)}</span>
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
  .list { max-width: 760px; display: grid; gap: 2px; }
  .row {
    display: flex; align-items: center; gap: 8px; width: 100%;
    min-height: 52px; padding: 8px; border-radius: var(--r-control);
  }
  .open {
    border: 0; background: none; text-align: left; cursor: pointer; font-size: 14px;
    transition: background-color 150ms var(--ease);
  }
  .open:hover { background: var(--fill); }
  .mark { display: inline-flex; justify-content: center; width: 16px; flex: none; }
  .main { flex: 1; min-width: 0; display: grid; }
  .title { font-weight: 500; }
  .count { display: inline-flex; align-items: center; gap: 4px; font-size: 12.5px; font-weight: 600; color: var(--text-2); }
  .state, .cause { font-size: 12.5px; color: var(--danger); }
  .cause { display: flex; align-items: center; gap: 6px; }
  .chevron { display: inline-flex; color: var(--text-2); }
  .empty { display: grid; justify-items: center; gap: 12px; padding: 64px 16px; }
</style>
