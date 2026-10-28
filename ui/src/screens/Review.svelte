<script lang="ts">
  import { tick } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { api, type MeetingDetail, type Resolution, type ReviewItem, type Segment } from '../lib/api';
  import { app, attempt, go, notify } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatDateTime, formatTimestamp } from '../lib/format';
  import { canSend, flaggedSegments, isOpen, isReadOnly, nextOpenAfter, remaining, segmentAt, speakerIds, speakerName, stepId } from '../lib/review';
  import Icon from '../lib/Icon.svelte';
  import Spark from '../lib/Spark.svelte';
  import ReviewZone from './ReviewZone.svelte';

  let { id }: { id: string } = $props();

  let detail = $state<MeetingDetail | null>(null);
  let currentTime = $state(0);
  let paused = $state(true);
  let duration = $state(0);
  let stopAt: number | null = null;
  let focusedId = $state<string | null>(null);
  let editing = $state<string | null>(null);
  let editText = $state('');
  let renames = $state<Record<string, string>>({});
  let sending = $state(false);

  async function load() {
    const d = await attempt(() => api.getMeeting(id));
    if (d) detail = d;
  }
  load();

  const readOnly = $derived(!!detail && isReadOnly(detail.meeting.state, detail.meeting.error));
  const back = $derived(readOnly ? 'home' : 'queue');
  const items = $derived(detail?.review_items ?? []);
  const segments = $derived(detail?.doc.segments ?? []);
  const participants = $derived(detail?.doc.meeting.participants ?? []);
  const flagged = $derived(flaggedSegments(items));
  const playing = $derived(paused ? null : segmentAt(segments, currentTime)?.id);
  const left = $derived(remaining(items));
  const current = $derived(items.find((i) => i.id === focusedId && isOpen(i))?.id ?? items.find(isOpen)?.id);
  const nameOf = (sid: string | null) => speakerName(sid, participants, t);
  const speakers = $derived(speakerIds(segments));
  const choices = $derived(
    [...new Set([...speakers, ...participants.map((p) => p.id)])].map((sid) => ({ id: sid, name: nameOf(sid) })),
  );

  $effect(() => {
    if (stopAt !== null && currentTime >= stopAt) {
      paused = true;
      stopAt = null;
    }
  });

  function seek(time: number, until: number | null = null) {
    if (!detail?.audio_path) return;
    currentTime = time;
    stopAt = until;
    paused = false;
  }

  function playZone(item: ReviewItem) {
    focusedId = item.id;
    document.getElementById(`seg-${item.segment_id}`)?.scrollIntoView({ block: 'nearest' });
    seek(item.start, item.end);
  }

  async function resolve(item: ReviewItem, r: Resolution) {
    if ((await attempt(() => api.resolveReviewItem(item.id, r))) === undefined) return;
    await load();
    const next = nextOpenAfter(items, item.id);
    await tick();
    if (next) focusZone(next);
  }

  function focusZone(zid: string) {
    focusedId = zid;
    document.getElementById(`zone-${zid}`)?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (readOnly || e.ctrlKey || e.metaKey || e.altKey || el.closest('input, textarea, select, [contenteditable="true"], dialog')) return;
    if (e.key !== 'j' && e.key !== 'k') return;
    e.preventDefault();
    const next = stepId(items, focusedId, e.key === 'j' ? 1 : -1);
    if (next) focusZone(next);
  }

  function startEdit(s: Segment) {
    if (readOnly) return;
    editing = s.id;
    editText = s.text;
  }

  async function saveEdit(s: Segment) {
    // Enter and the blur that follows both land here: save once.
    if (editing !== s.id) return;
    const text = editText.trim();
    editing = null;
    if (text === s.text) return;
    if ((await attempt(() => api.updateSegmentText(id, s.id, text))) !== undefined) s.text = text;
  }

  function editKeys(e: KeyboardEvent, s: Segment) {
    if (e.key === 'Escape') editing = null;
    else if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      saveEdit(s);
    }
  }

  async function rename(sid: string) {
    const name = renames[sid]?.trim();
    if (!name) return;
    if ((await attempt(() => api.renameSpeaker(id, sid, name))) === undefined) return;
    delete renames[sid];
    await load();
  }

  async function exportMeeting() {
    const start = app.settings?.destinations.folder.path || (await api.defaultFolder().catch(() => ''));
    const dir = await api.pickFolder(start).catch(() => null);
    if (dir && (await attempt(() => api.exportMeeting(id, dir))) !== undefined) notify(t('review.exported'));
  }

  async function send() {
    sending = true;
    if ((await attempt(() => api.validateReview(id))) !== undefined) {
      notify(t('review.sent'));
      go({ name: 'queue' });
    }
    sending = false;
  }
</script>

<svelte:window {onkeydown} />

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <button class="btn ghost" onclick={() => go({ name: back })}>{t('common.back')}</button>
    <div class="title">
      <h1>{detail?.meeting.title ?? ''}</h1>
      {#if detail}
        <p class="secondary">{formatDateTime(detail.meeting.started_at, app.lang)}{#if !readOnly}{' · '}{t('review.progress', { done: items.length - left, total: items.length })}{/if}</p>
      {/if}
    </div>
    {#if readOnly}
      <button class="btn" onclick={exportMeeting}>{t('review.export')}</button>
    {:else}
      {#if left > 0}<span id="send-hint" class="secondary">{t('review.remaining', { n: left })}</span>{/if}
      <button class="btn primary" disabled={!detail || !canSend(items) || sending} aria-describedby={left > 0 ? 'send-hint' : undefined} onclick={send}>
        {t('review.send')}
      </button>
    {/if}
  </header>

  {#if detail}
    <div class="player">
      {#if detail.audio_path}
        {@const length = duration || 0}
        <audio preload="metadata" src={convertFileSrc(detail.audio_path)} bind:currentTime bind:paused bind:duration></audio>
        <button class="btn icon" aria-label={t(paused ? 'review.play' : 'review.pause')} onclick={() => (paused = !paused)}>
          <Icon name={paused ? 'play' : 'pause'} />
        </button>
        <span class="time secondary tabular">{formatTimestamp(currentTime)} / {formatTimestamp(length)}</span>
        <div class="track">
          <input type="range" min="0" max={length} step="1" bind:value={currentTime} aria-label={t('review.audio')} aria-valuetext={formatTimestamp(currentTime)} style:--p={length ? currentTime / length : 0} />
          <!-- The zones to review, marked on the timeline. -->
          {#if length}{#each items as item (item.id)}<span class="tick" style:--at={Math.min(1, item.start / length)}><Spark size={8} active={isOpen(item)} /></span>{/each}{/if}
        </div>
      {:else}
        <p class="secondary">{t('review.no_audio')}</p>
      {/if}
    </div>

    <div class="columns" class:single={readOnly}>
      <div class="col transcript">
        {#if detail.doc.summary}
          {@const s = detail.doc.summary}
          <!-- A heading inside <summary> loses its role: the hidden one keeps heading navigation. -->
          <h2 class="sr-only">{t('review.summary')}</h2>
          <details class="card summary">
            <summary><span class="chev"><Icon name="chevron" /></span>{t('review.summary')}</summary>
            <p>{s.overview}</p>
            {#if s.decisions.length}<h3>{t('review.decisions')}</h3><ul>{#each s.decisions as d}<li>{d}</li>{/each}</ul>{/if}
            {#if s.action_items.length}
              <h3>{t('review.actions')}</h3>
              <ul>{#each s.action_items as a}<li>{a.task}{#if a.owner}{' · '}<strong>{a.owner}</strong>{/if}{#if a.due}{` · ${a.due}`}{/if}</li>{/each}</ul>
            {/if}
            {#if s.open_questions.length}<h3>{t('review.questions')}</h3><ul>{#each s.open_questions as q}<li>{q}</li>{/each}</ul>{/if}
          </details>
        {/if}

        <h2 class="sr-only">{t('review.transcript')}</h2>
        <ol class="segments">
          {#each segments as s (s.id)}
            <li id="seg-{s.id}" class="seg" class:flagged={flagged.has(s.id)} class:playing={playing === s.id}>
              <span class="mark"><Spark active={flagged.has(s.id)} /></span>
              <button class="ts tabular hit" disabled={!detail.audio_path} aria-label={t('review.seek', { time: formatTimestamp(s.start) })} onclick={() => seek(s.start)}>
                {formatTimestamp(s.start)}
              </button>
              <div class="body">
                <span class="chip" class:anon={!participants.some((p) => p.id === s.speaker_id) && s.speaker_id !== 'me'}>{nameOf(s.speaker_id)}</span>
                {#if editing === s.id}
                  <!-- svelte-ignore a11y_autofocus -->
                  <textarea rows="3" bind:value={editText} autofocus onkeydown={(e) => editKeys(e, s)} onblur={() => saveEdit(s)}></textarea>
                {:else}
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <p class="text" ondblclick={() => startEdit(s)}>{s.text || '…'}</p>
                  {#if !readOnly}
                  <button class="btn ghost icon edit" aria-label={t('review.edit', { time: formatTimestamp(s.start) })} title={t('review.edit', { time: formatTimestamp(s.start) })} onclick={() => startEdit(s)}>
                    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M10.5 3.5l2 2L6 12H4v-2zM9.5 4.5l2 2" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" /></svg>
                  </button>
                  {/if}
                {/if}
              </div>
            </li>
          {/each}
        </ol>
      </div>

      {#if !readOnly}
      <aside class="col side">
        <section aria-labelledby="zones-title">
          <div class="side-head">
            <h2 id="zones-title">{t('review.zones')}</h2>
            <span class="secondary tabular">{items.length - left}/{items.length}</span>
          </div>
          {#if items.length === 0}
            <p class="secondary">{t('review.zones_none')}</p>
          {:else}
            <p class="secondary keys">{t('review.keys')}</p>
            <ul class="zones">
              {#each items as item (item.id)}
                <ReviewZone {item} speakers={choices} current={item.id === current} onresolve={(r) => resolve(item, r)} onplay={() => playZone(item)} onfocus={() => (focusedId = item.id)} />
              {/each}
            </ul>
          {/if}
        </section>

        <section class="speakers" aria-labelledby="speakers-title">
          <h2 id="speakers-title">{t('review.speakers')}</h2>
          <datalist id="participant-names">{#each participants as p (p.id)}<option value={p.name}></option>{/each}</datalist>
          <ul>
            {#each speakers.filter((sid) => sid !== 'me') as sid (sid)}
              <li>
                <form onsubmit={(e) => { e.preventDefault(); rename(sid); }}>
                  <label class="field">
                    <span>{t('review.rename', { speaker: nameOf(sid) })}</span>
                    <input type="text" list="participant-names" value={renames[sid] ?? ''} oninput={(e) => (renames[sid] = e.currentTarget.value)} placeholder={nameOf(sid)} autocomplete="off" />
                  </label>
                  <button class="btn" type="submit" disabled={!renames[sid]?.trim()}>{t('review.rename_apply')}</button>
                </form>
              </li>
            {/each}
          </ul>
        </section>
      </aside>
      {/if}
    </div>
  {/if}
</section>

<style>
  .title { flex: 1; min-width: 0; }
  .title h1 { white-space: normal; overflow-wrap: anywhere; }
  .player { display: flex; align-items: center; gap: 12px; padding: 0 40px 16px; }
  .time { flex: none; min-width: 88px; }
  .track { position: relative; flex: 1; min-width: 0; display: grid; }
  .track input { width: 100%; }
  /* Same inset as the thumb, so a tick sits where the thumb lands. */
  .tick { position: absolute; top: 50%; left: calc(7px + (100% - 14px) * var(--at)); translate: -50% -50%; display: inline-flex; pointer-events: none; }

  .columns { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr) 360px; border-top: 1px solid var(--border); }
  .columns.single { grid-template-columns: minmax(0, 1fr); }
  .col { overflow: auto; min-height: 0; }
  .transcript { padding: 24px 24px 40px 32px; }
  .side { padding: 24px 24px 40px; border-left: 1px solid var(--border); display: grid; gap: 32px; align-content: start; }

  /* Reading column: about 68 characters of text beside the spark and timestamp gutter. */
  .summary, .segments { max-width: calc(68ch + 100px); }
  .summary { padding: 12px 16px; margin: 0 0 24px 8px; display: grid; gap: 8px; }
  .summary summary { cursor: pointer; list-style: none; display: flex; align-items: center; gap: 6px; }
  .summary summary::-webkit-details-marker { display: none; }
  .chev { display: inline-flex; color: var(--text-2); transition: transform 150ms var(--ease); }
  .summary[open] .chev { transform: rotate(90deg); }
  .summary summary { font-size: 15px; font-weight: 650; line-height: 1.35; }
  .summary h3 { font-size: 12.5px; font-weight: 650; color: var(--text-2); margin-top: 4px; }
  .summary ul { list-style: disc; padding-left: 20px; }

  .segments { display: grid; gap: 2px; }
  .seg {
    display: grid; grid-template-columns: 16px 52px minmax(0, 1fr); gap: 8px;
    padding: 8px; border-radius: var(--r-control);
    transition: background-color 150ms var(--ease), box-shadow 150ms var(--ease);
  }
  .seg.flagged { background: var(--fill-strong); }
  .seg.playing { box-shadow: inset 0 0 0 2px var(--spark); }
  .mark { display: flex; justify-content: center; padding-top: 4px; }
  .ts {
    align-self: start; padding: 2px 4px; border: 0; border-radius: 4px; background: none;
    font-size: 12.5px; color: var(--text-2); cursor: pointer;
  }
  .ts:hover:not(:disabled) { color: var(--accent-text); }
  .ts:disabled { cursor: default; }
  .body { position: relative; min-width: 0; padding-right: 32px; }
  .chip { display: block; margin-bottom: 2px; font-size: 12.5px; font-weight: 650; }
  .chip.anon { font-weight: 500; color: var(--text-2); }
  .text { white-space: pre-wrap; line-height: 1.6; }
  .body textarea { margin-top: 2px; }
  .edit { position: absolute; top: -4px; right: 0; opacity: 0; transition: opacity 150ms var(--ease); }
  .seg:hover .edit, .edit:focus-visible { opacity: 1; }
  @media (hover: none) { .edit { opacity: 1; } }

  .side-head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 4px; }
  .keys { margin-bottom: 12px; }
  .zones { display: grid; gap: 4px; }
  .speakers { display: grid; gap: 8px; }
  .speakers ul { display: grid; gap: 12px; }
  .speakers form { display: grid; grid-template-columns: 1fr auto; gap: 8px; align-items: end; }

  @media (max-width: 1000px) {
    .columns { grid-template-columns: minmax(0, 1fr) 260px; }
    .transcript { padding: 16px 12px 32px 16px; }
    .side { padding: 16px 16px 32px; }
    .player { padding-inline: 24px; }
    .seg { grid-template-columns: 12px 44px minmax(0, 1fr); gap: 4px; padding-inline: 4px; }
    .body { padding-right: 28px; }
  }
</style>
