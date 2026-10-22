<script lang="ts">
  import type { Resolution, ReviewItem } from '../lib/api';
  import { t } from '../lib/i18n';
  import { formatTimestamp } from '../lib/format';
  import { wordDiff } from '../lib/review';
  import Icon from '../lib/Icon.svelte';
  import Spark from '../lib/Spark.svelte';

  // Only the current zone shows its question and actions: one decision at a time.
  let { item, speakers, current, onresolve, onplay, onfocus }: {
    item: ReviewItem;
    speakers: { id: string; name: string }[];
    current: boolean;
    onresolve: (r: Resolution) => void;
    onplay: () => void;
    onfocus: () => void;
  } = $props();

  let mode = $state<'none' | 'correct' | 'reassign'>('none');
  let text = $state('');
  let speaker = $state('');

  const resolved = $derived(item.resolution !== null);

  // A reopened zone starts closed: an autofocused textarea would swallow j/k.
  $effect(() => {
    if (!current) mode = 'none';
  });

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
  class="zone"
  class:current={current && !resolved}
  class:resolved
  tabindex="0"
  aria-labelledby="zone-kind-{item.id} zone-q-{item.id}"
  {onkeydown}
  {onclick}
  onfocusin={onfocus}
>
  <div class="head">
    <span class="mark">
      {#if resolved}<span class="ok"><Icon name="check" size={14} /></span>{/if}
      <Spark active={!resolved} />
    </span>
    <span class="kind" id="zone-kind-{item.id}">{t(`review.kind.${item.kind}`)}</span>
    <button class="btn ghost small tabular" aria-label={t('review.seek', { time: formatTimestamp(item.start) })} onclick={onplay}>
      <Icon name="play" size={10} />{formatTimestamp(item.start)}
    </button>
  </div>

  {#if resolved}
    <p class="done">{summary(item.resolution!)}</p>
  {:else if current}
    <div class="detail">
      <p class="question" id="zone-q-{item.id}">{t(`review.q.${item.kind}`)}</p>
      {#if item.proposed_text}
        <blockquote class="proposed" aria-label={t('review.proposed')}>{item.proposed_text}</blockquote>
      {/if}
      {#if item.alternatives.length > 0}
        <div class="alts" role="group" aria-labelledby="zone-alts-{item.id}">
          <p class="secondary" id="zone-alts-{item.id}">{t('review.maybe')}</p>
          {#each item.alternatives as alt (alt)}
            <button class="btn small alt" onclick={() => onresolve({ action: 'correct', text: alt })}>
              <span>{#each wordDiff(item.proposed_text, alt) as part, i (i)}{#if part.changed}<strong>{part.text}</strong>{:else}{part.text}{/if}{/each}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="actions">
        <button class="btn small validate" onclick={() => onresolve({ action: 'validate' })}><Icon name="check" size={14} />{t('review.validate')}</button>
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
    </div>
  {/if}
</li>

<style>
  /* Quiet rows; the current zone opens into the panel's only card. */
  .zone {
    display: grid; gap: 8px; padding: 6px 8px; cursor: default;
    border: 1px solid transparent; border-radius: var(--r-card);
    transition: background-color 150ms var(--ease), border-color 150ms var(--ease);
  }
  .zone:not(.current):hover { background: var(--fill); }
  .zone:focus-visible { outline-offset: 0; }
  .current {
    padding: 12px; background: var(--surface); border-color: var(--border);
    box-shadow: var(--shadow-float);
  }
  .head { display: flex; align-items: center; gap: 8px; min-height: 28px; }
  .mark { display: grid; place-items: center; width: 16px; flex: none; }
  .mark > :global(*) { grid-area: 1 / 1; }
  .ok { display: inline-flex; color: var(--ok); }
  .kind { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 600; }
  .resolved .kind { color: var(--text-2); font-weight: 500; }
  .head .btn { gap: 4px; color: var(--text-2); }
  .detail { display: grid; gap: 12px; padding-left: 24px; }
  .question { font-size: 15px; font-weight: 600; line-height: 1.35; }
  .proposed {
    margin: 0; padding: 8px 12px; border-radius: var(--r-control);
    background: var(--fill); font-size: 14px;
  }
  .alts { display: flex; flex-wrap: wrap; gap: 4px; }
  .alts p { width: 100%; }
  .alt { white-space: normal; text-align: left; font-weight: 400; color: var(--text-2); }
  .alt strong { font-weight: 650; color: var(--text); }
  .actions { display: flex; flex-wrap: wrap; gap: 4px; }
  .validate { gap: 4px; color: var(--accent-text); border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .extra { display: grid; gap: 8px; justify-items: start; }
  .extra .field { width: 100%; }
  .extra.row { grid-template-columns: 1fr auto; align-items: end; }
  .grow { min-width: 0; }
  .done { padding-left: 24px; font-size: 12.5px; color: var(--text-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
