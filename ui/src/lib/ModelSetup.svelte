<script lang="ts">
  // First-run pack: offers every model the current settings need, downloaded one after another.
  import { onMount } from 'svelte';
  import { api, events, type ModelInfo } from './api';
  import { t } from './i18n';
  import { formatSize } from './format';
  import { missingRequired } from './models';
  import { app, attempt, notify } from './state.svelte';
  import Sheet from './Sheet.svelte';

  let missing = $state<ModelInfo[]>([]);
  let open = $state(false);
  let busy = $state(false);
  let progress = $state<Record<string, number>>({});

  const size = (mb: number) => {
    const s = formatSize(mb, app.lang);
    return t(`settings.size_${s.unit}`, { n: s.n });
  };
  const total = $derived(missing.reduce((sum, m) => sum + m.size_mb, 0));

  onMount(() => {
    (async () => {
      const models = await attempt(api.listModels);
      if (!models || !app.settings) return;
      missing = missingRequired(models, app.settings);
      open = missing.length > 0;
    })();
    const off = events.modelProgress(({ id, progress: p }) => (progress[id] = p));
    return () => off.then((f) => f());
  });

  async function downloadAll() {
    busy = true;
    for (const m of missing) {
      if (progress[m.id] === 1) continue;
      progress[m.id] = 0;
      // Stop at the first failure: the error is already shown, the rest can wait for a retry.
      if ((await attempt(() => api.downloadModel(m.id))) === undefined) {
        busy = false;
        return;
      }
      progress[m.id] = 1;
    }
    busy = false;
    open = false;
    notify(t('setup.done'));
  }
</script>

<Sheet {open} title={t('setup.title')} onclose={() => !busy && (open = false)}>
  <p>{t('setup.intro')}</p>
  <ul>
    {#each missing as m (m.id)}
      <li>
        <span class="name">{m.name}<small>{t(`setup.role.${m.kind}`)}</small></span>
        <span class="state tabular">
          {#if progress[m.id] === 1}{t('settings.installed')}
          {:else if progress[m.id] !== undefined}{t('settings.downloading', { pct: Math.round(progress[m.id] * 100) })}
          {:else}{size(m.size_mb)}{/if}
        </span>
      </li>
    {/each}
  </ul>
  <p class="total">{t('setup.total', { size: size(total) })}</p>
  {#snippet actions()}
    <button class="btn" disabled={busy} onclick={() => (open = false)}>{t('setup.later')}</button>
    <button class="btn primary" disabled={busy} onclick={downloadAll}>{t('setup.download_all')}</button>
  {/snippet}
</Sheet>

<style>
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  li { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .name { display: grid; }
  small { color: var(--text-2); font-size: 12.5px; }
  .state { color: var(--text-2); font-size: 12.5px; white-space: nowrap; }
  .total { color: var(--text-2); font-size: 12.5px; }
</style>
