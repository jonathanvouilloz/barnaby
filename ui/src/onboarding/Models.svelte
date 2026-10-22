<script lang="ts">
  // Where meetings are transcribed, then the models that choice needs, downloaded one after another.
  import { onMount, tick } from 'svelte';
  import { api, events, type ModelInfo } from '../lib/api';
  import { t } from '../lib/i18n';
  import { formatSize } from '../lib/format';
  import { cloudSummary, modeFor } from '../lib/settings';
  import { app, applySettings, attempt } from '../lib/state.svelte';
  import MistralSheet from '../lib/MistralSheet.svelte';
  import { meetingModelFor, modelsFor } from './steps';

  type Choice = 'local' | 'cloud';

  // busy: the parent keeps the step open while a download runs (leaving would orphan the loop).
  let { ready = $bindable(), busy = $bindable(false) }: { ready: boolean; busy?: boolean } = $props();
  ready = false;

  let models = $state<ModelInfo[]>([]);
  let choice = $state<Choice | null>(null);
  let mistralOpen = $state(false);
  let hasKey = $state(false);
  let progress = $state<Record<string, number>>({});
  let failed = $state(false);
  let announce = $state('');

  // The catalog is read once: models installed here stay listed, marked done.
  const list = $derived(choice ? modelsFor(choice, models, app.settings!) : []);
  const total = $derived(list.reduce((sum, m) => sum + m.size_mb, 0));
  const allIn = $derived(list.every((m) => progress[m.id] === 1));
  $effect(() => {
    ready = choice !== null && allIn;
  });

  onMount(() => {
    attempt(api.listModels).then((v) => (models = v ?? []));
    const off = events.modelProgress(({ id, progress: p }) => {
      if (progress[id] !== 1) progress[id] = p;
    });
    return () => off.then((f) => f());
  });

  async function pick(c: Choice) {
    // The disclosure shows on every cloud choice, even with a saved key.
    if (c === 'cloud') {
      hasKey = (await attempt(api.hasMistralApiKey)) ?? false;
      mistralOpen = true;
      return;
    }
    await save(c);
  }

  // Writes onto the saved settings, never onto a stale copy.
  async function save(c: Choice) {
    const s = $state.snapshot(app.settings!);
    const model = meetingModelFor(c, s);
    const next = { ...s, meeting: { ...s.meeting, model }, processing_mode: modeFor(model, cloudSummary(s)) };
    if ((await attempt(() => api.saveSettings(next))) === undefined) return;
    applySettings(next);
    choice = c;
  }

  async function download() {
    busy = true;
    failed = false;
    for (const m of list) {
      if (progress[m.id] === 1) continue;
      progress[m.id] = 0;
      // Stop at the first failure: the rest waits for « Réessayer ».
      if ((await attempt(() => api.downloadModel(m.id))) === undefined) {
        delete progress[m.id];
        failed = true;
        break;
      }
      progress[m.id] = 1;
      announce = t('onboarding.models.installed', { name: m.name });
    }
    busy = false;
    if (failed) return;
    announce = t('onboarding.models.all_in');
    // The download button goes away: keep the keyboard on the way forward, not on the page body.
    await tick();
    document.getElementById('onboarding-continue')?.focus();
  }

  const size = (mb: number) => {
    const s = formatSize(mb, app.lang);
    return t(`settings.size_${s.unit}`, { n: s.n });
  };
</script>

<p class="lead">{t('onboarding.models.text')}</p>

<div class="choices" role="group" aria-label={t('onboarding.models.title')}>
  {#each ['local', 'cloud'] as const as c (c)}
    <button class="choice card" aria-pressed={choice === c} disabled={busy} onclick={() => pick(c)}>
      <span class="head">
        <span class="name">{t(`onboarding.models.${c}`)}</span>
        {#if c === 'local'}<span class="badge">{t('onboarding.models.recommended')}</span>{/if}
      </span>
      <span class="secondary">{t(`onboarding.models.${c}_text`)}</span>
    </button>
  {/each}
</div>

{#if choice}
  {#if list.length}
    <div class="download">
      <ul>
        {#each list as m (m.id)}
          {@const p = progress[m.id]}
          <li>
            <span class="name">{m.name}<span class="secondary">{t(`settings.kind.${m.kind}`)}</span></span>
            <span class="state secondary tabular">
              {#if p === 1}<span class="ok">{t('settings.installed')}</span>
              {:else if p >= 0.99}{t('settings.installing')}
              {:else if p !== undefined}{t('settings.downloading', { pct: Math.round(p * 100) })}
              {:else}{size(m.size_mb)}{/if}
            </span>
            {#if p !== undefined && p < 1}<progress max="1" value={p} aria-label={m.name}></progress>{/if}
          </li>
        {/each}
      </ul>
      {#if failed}<p class="problem" role="alert">{t('onboarding.models.failed')}</p>{/if}
      {#if !allIn}
        <!-- aria-disabled, not disabled: a disabled button would drop the focus during the download. -->
        <button class="btn" aria-disabled={busy} onclick={() => busy || download()}>
          {failed ? t('common.retry') : t('onboarding.models.download', { size: size(total) })}
        </button>
      {/if}
      <p class="sr-only" aria-live="polite">{announce}</p>
    </div>
  {:else}
    <p class="secondary" role="status">{t('onboarding.models.ready')}</p>
  {/if}
{/if}

<MistralSheet open={mistralOpen} {hasKey} oncancel={() => (mistralOpen = false)} onconfirm={() => { mistralOpen = false; save('cloud'); }} />

<style>
  .lead { font-size: 16px; }
  .choices { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; width: 100%; }
  .choice {
    display: grid; gap: 4px; align-content: start; padding: 16px; text-align: left; cursor: pointer;
    border-color: var(--border-strong);
    transition: border-color 150ms var(--ease), background-color 150ms var(--ease);
  }
  .choice:hover:not(:disabled) { background: var(--fill); }
  .choice[aria-pressed="true"] { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
  .choice:disabled { cursor: default; }
  .name { font-weight: 600; display: grid; }
  .head { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 4px 8px; }
  .badge {
    padding: 2px 8px; border-radius: 999px; font-size: 12px; font-weight: 600;
    background: var(--fill-strong); color: var(--text);
  }
  .download { display: grid; gap: 12px; width: 100%; justify-items: start; }
  ul { display: grid; gap: 8px; width: 100%; }
  li { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 4px 12px; }
  .name .secondary { font-weight: 400; }
  .state { white-space: nowrap; }
  .ok { color: var(--ok); }
  progress { grid-column: 1 / -1; width: 100%; height: 4px; accent-color: var(--accent); }
  .problem { color: var(--danger); }
  [aria-disabled="true"] { opacity: .5; pointer-events: none; }
  @media (max-width: 560px) { .choices { grid-template-columns: 1fr; } }
</style>
