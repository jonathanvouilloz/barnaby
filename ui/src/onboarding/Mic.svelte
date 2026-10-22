<script lang="ts">
  // Live mic level; nothing is recorded. The test must stop whenever this step goes away.
  import { onMount } from 'svelte';
  import { api, events } from '../lib/api';
  import { t } from '../lib/i18n';
  import Spark from '../lib/Spark.svelte';

  let { ready = $bindable() }: { ready: boolean } = $props();
  ready = false;

  let level = $state(0);
  let heard = $state(false);
  let error = $state('');
  let alive = true;

  async function start() {
    error = '';
    try {
      await api.startMicTest();
      // Left the step while the mic was opening: close it again.
      if (!alive) api.stopMicTest();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    const off = events.micLevel((l) => {
      level = l;
      if (l > 0.3 && !heard) heard = ready = true;
    });
    start();
    return () => {
      alive = false;
      api.stopMicTest().catch(() => {});
      off.then((f) => f());
    };
  });
</script>

<p class="lead">{t('onboarding.mic.text')}</p>

{#if error}
  <div class="problem" role="alert">
    <p>{error === 'capture.permission_denied' ? t(error) : t('onboarding.mic.failed')}</p>
    <p class="secondary">{t('onboarding.mic.help')}</p>
    <button class="btn" onclick={start}>{t('common.retry')}</button>
  </div>
{:else}
  <div class="meter">
    <span class="bar" role="meter" aria-label={t('onboarding.mic.level')} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(level * 100)}>
      <span class="fill" style:transform="scaleX({level})"></span>
    </span>
    <span class="heard" aria-live="polite">
      <Spark size={16} active={!heard} />
      {#if heard}{t('onboarding.mic.heard')}{/if}
    </span>
  </div>
{/if}

<style>
  .lead { font-size: 16px; }
  .meter { display: flex; align-items: center; gap: 16px; width: 100%; min-height: 32px; }
  .bar { flex: 1; height: 8px; border-radius: 4px; background: var(--fill-strong); overflow: hidden; }
  .fill {
    display: block; height: 100%; background: var(--accent);
    transform-origin: left; transition: transform 80ms linear;
  }
  .heard { display: inline-flex; align-items: center; gap: 8px; min-width: 120px; font-weight: 550; color: var(--accent-text); }
  .problem { display: grid; gap: 8px; justify-items: start; }
  .problem > p:first-child { color: var(--danger); }
</style>
