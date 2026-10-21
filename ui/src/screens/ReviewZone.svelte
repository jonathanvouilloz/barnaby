<script lang="ts">
  import type { Resolution, ReviewItem } from '../lib/api';
  import { t } from '../lib/i18n';
  import { formatTimestamp } from '../lib/format';

  let { item, speakers, onresolve, onplay }: {
    item: ReviewItem;
    speakers: { id: string; name: string }[];
    onresolve: (r: Resolution) => void;
    onplay: () => void;
  } = $props();

  let mode = $state<'none' | 'correct' | 'reassign'>('none');
  let text = $state('');
  let speaker = $state('');

  const resolved = $derived(item.resolution !== null);

  function summary(r: Resolution): string {
    if (r.action === 'correct') return t('review.res.correct', { text: r.text });
    if (r.action === 'reassign') return t('review.res.reassign', { name: speakers.find((s) => s.id === r.speaker_id)?.name ?? r.speaker_id });
    return t(`review.res.${r.action}`);
  }

  function startCorrect() {
    text = item.proposed_text;
    mode = mode === 'correct' ? 'none' : 'correct';
  }

  function startReassign() {
    speaker = speakers[0]?.id ?? '';
    mode = mode === 'reassign' ? 'none' : 'reassign';
  }

  function onkeydown(e: KeyboardEvent) {
    // Enter on the card itself, not inside a field or on one of its buttons.
    if (e.key === 'Enter' && e.target === e.currentTarget && !resolved) {
      e.preventDefault();
      onresolve({ action: 'validate' });
    }
  }

  function onclick(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('button, textarea, select, input')) return;
    onplay();
  }
</script>

<!-- The card is focusable for j/k navigation; every action inside stays a real button. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions, a11y_click_events_have_key_events -->
<li
  id="zone-{item.id}"
  class="zone card"
  class:resolved
  tabindex="0"
  aria-label="{t(`review.kind.${item.kind}`)}, {formatTimestamp(item.start)}"
  {onkeydown}
  {onclick}
>
  <div class="head">
    <span class="kind">{t(`review.kind.${item.kind}`)}</span>
    <button class="btn ghost small tabular" aria-label={t('review.seek', { time: formatTimestamp(item.start) })} onclick={onplay}>
      ▶ {formatTimestamp(item.start)}
    </button>
  </div>

  {#if resolved}
    <p class="done"><span aria-hidden="true">✓</span> {summary(item.resolution!)}</p>
  {:else}
    <p class="question">{t(`review.q.${item.kind}`)}</p>
    {#if item.proposed_text}
      <blockquote class="proposed" aria-label={t('review.proposed')}>{item.proposed_text}</blockquote>
    {/if}
    {#if item.alternatives.length > 0}
      <div class="alts" role="group" aria-label={t('review.alternatives')}>
        {#each item.alternatives as alt (alt)}
          <button class="btn small alt" onclick={() => onresolve({ action: 'correct', text: alt })}>{alt}</button>
        {/each}
      </div>
    {/if}

    <div class="actions">
      <button class="btn small validate" onclick={() => onresolve({ action: 'validate' })}>{t('review.validate')}</button>
      <button class="btn small" aria-expanded={mode === 'correct'} onclick={startCorrect}>{t('review.correct')}</button>
      <button class="btn small" aria-expanded={mode === 'reassign'} onclick={startReassign}>{t('review.reassign')}</button>
      <button class="btn small" onclick={() => onresolve({ action: 'mark_inaudible' })}>{t('review.inaudible')}</button>
      <button class="btn ghost small" onclick={() => onresolve({ action: 'ignore' })}>{t('review.ignore')}</button>
    </div>

    {#if mode === 'correct'}
      <form class="extra" onsubmit={(e) => { e.preventDefault(); if (text.trim()) onresolve({ action: 'correct', text: text.trim() }); }}>
        <label class="field"><span>{t('review.correction')}</span>
          <!-- svelte-ignore a11y_autofocus -->
          <textarea rows="3" bind:value={text} autofocus></textarea>
        </label>
        <button class="btn small" type="submit" disabled={!text.trim()}>{t('review.apply')}</button>
      </form>
    {:else if mode === 'reassign'}
      <form class="extra row" onsubmit={(e) => { e.preventDefault(); if (speaker) onresolve({ action: 'reassign', speaker_id: speaker }); }}>
        <label class="field grow"><span>{t('review.speaker')}</span>
          <select bind:value={speaker}>
            {#each speakers as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
          </select>
        </label>
        <button class="btn small" type="submit">{t('review.apply')}</button>
      </form>
    {/if}
  {/if}
</li>

<style>
  .zone {
    padding: 12px 14px; display: grid; gap: 8px; cursor: default;
    border-left: 3px solid var(--warn);
    transition: border-color 150ms var(--ease), opacity 150ms var(--ease);
  }
  .zone:focus-visible { outline-offset: 1px; }
  .zone:focus-within:not(.resolved) { box-shadow: 0 0 0 1px var(--accent); }
  .resolved { border-left-color: var(--ok); padding-block: 8px; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .kind { font-size: 12.5px; font-weight: 600; color: var(--warn); }
  .resolved .kind { color: var(--text-2); font-weight: 500; }
  .question { font-weight: 500; }
  .proposed {
    margin: 0; padding: 8px 10px; border-radius: var(--r-control);
    background: var(--fill); font-size: 13.5px;
  }
  .alts { display: flex; flex-wrap: wrap; gap: 6px; }
  .alt { white-space: normal; text-align: left; }
  .actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .validate { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .extra { display: grid; gap: 8px; justify-items: start; }
  .extra .field { width: 100%; }
  .extra.row { grid-template-columns: 1fr auto; align-items: end; }
  .done { font-size: 13px; color: var(--text-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .done span { color: var(--ok); }
</style>
