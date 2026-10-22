<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './lib/api';
  import { app, applySettings, attempt, go, init, reviewCount, type Route } from './lib/state.svelte';
  import { missingRequired } from './lib/models';
  import { t } from './lib/i18n';
  import Icon, { type IconName } from './lib/Icon.svelte';
  import Spark from './lib/Spark.svelte';
  import Home from './screens/Home.svelte';
  import NewMeeting from './screens/NewMeeting.svelte';
  import Recording from './screens/Recording.svelte';
  import ReviewQueue from './screens/ReviewQueue.svelte';
  import Review from './screens/Review.svelte';
  import Settings from './screens/Settings.svelte';
  import Onboarding from './onboarding/Onboarding.svelte';
  import { shouldOnboard } from './onboarding/steps';

  // Nothing is drawn until the first-run decision, so home never flashes before the onboarding.
  let started = $state(false);

  onMount(async () => {
    await init();
    const s = app.settings;
    const models = s && !s.onboarding_done ? await attempt(api.listModels) : undefined;
    if (s && models) {
      const decision = shouldOnboard(s, missingRequired(models, s));
      if (decision === 'show' && !app.recordingId) go({ name: 'onboarding' });
      if (decision === 'mark_done') {
        const next = { ...$state.snapshot(s), onboarding_done: true };
        // Silent: an existing install has nothing to see; a failure just asks again next launch.
        api.saveSettings(next).then(() => applySettings(next), () => {});
      }
    }
    started = true;
  });

  const nav: { route: Route['name']; key: string; icon: IconName; match: Route['name'][] }[] = [
    { route: 'home', key: 'nav.home', icon: 'home', match: ['home', 'new', 'recording'] },
    { route: 'queue', key: 'nav.review', icon: 'review', match: ['queue', 'review'] },
    { route: 'settings', key: 'nav.settings', icon: 'settings', match: ['settings'] },
  ];

  const status = $derived(
    app.dictationStatus !== 'idle' ? app.dictationStatus : app.recordingId ? 'meeting' : 'idle',
  );
</script>

{#snippet notice()}
  <p class="notice" role="status" aria-live="polite">{#if app.notice}<span>{app.notice}</span>{/if}</p>
{/snippet}

{#if !started}
  <!-- first-run decision pending -->
{:else if app.route.name === 'onboarding'}
  <main class="solo">
    {#if app.settings}<Onboarding />{/if}
    {@render notice()}
  </main>
{:else}
<div class="shell">
  <aside class="sidebar">
    <div class="brand" data-tauri-drag-region>Barnaby <Spark size={10} /></div>
    <nav aria-label={t('nav.label')}>
      <ul>
        {#each nav as item (item.route)}
          {@const current = item.match.includes(app.route.name)}
          <li>
            <button
              class="nav-item"
              class:current
              aria-current={current ? 'page' : undefined}
              onclick={() => go({ name: item.route } as Route)}
            >
              <span class="icon {item.icon}"><Icon name={item.icon} /></span>
              <span class="label">{t(item.key)}</span>
              {#if item.route === 'queue' && reviewCount() > 0}
                <span class="count tabular" aria-label={t('nav.review_badge', { n: reviewCount() })}><Spark size={10} />{reviewCount()}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    </nav>

    <div class="foot">
      <p class="pill {status}" aria-live="polite">
        <span class="dot" aria-hidden="true"></span>{t(`status.${status}`)}
      </p>
      {#if app.recordingId}
        <button class="btn new live" onclick={() => go({ name: 'recording' })}><span class="rec" aria-hidden="true"></span>{t('rec.live')}</button>
      {:else}
        <button class="btn new" onclick={() => go({ name: 'new' })}><span class="rec" aria-hidden="true"></span>{t('new.title')}</button>
      {/if}
    </div>
  </aside>

  <main>
    {#if app.settings}
      {#if app.route.name === 'home'}<Home />
      {:else if app.route.name === 'new'}<NewMeeting />
      {:else if app.route.name === 'recording'}<Recording />
      {:else if app.route.name === 'queue'}<ReviewQueue />
      {:else if app.route.name === 'review'}{#key app.route.id}<Review id={app.route.id} />{/key}
      {:else if app.route.name === 'settings'}<Settings />
      {/if}
    {/if}
    {@render notice()}
  </main>
</div>
{/if}

<style>
  .shell { display: grid; grid-template-columns: 232px 1fr; height: 100%; }

  /* The only translucent surface: navigation, never content. */
  .sidebar {
    display: flex; flex-direction: column;
    background: var(--sidebar);
    backdrop-filter: blur(30px) saturate(1.8);
    border-right: 1px solid var(--border);
    padding: 0 12px 16px;
    min-height: 0;
  }
  .brand {
    display: flex; align-items: center; gap: 4px;
    padding: 24px 8px 20px; font-weight: 700; font-size: 15px;
  }
  nav ul { display: grid; gap: 4px; }
  .nav-item {
    display: flex; align-items: center; gap: 8px; width: 100%;
    min-height: 32px; padding: 4px 8px; border: 0; border-radius: var(--r-control);
    background: transparent; text-align: left; cursor: pointer; font-size: 14px; font-weight: 500;
    transition: background-color 150ms var(--ease);
  }
  .nav-item:hover { background: var(--fill); }
  .nav-item.current { background: var(--fill-strong); font-weight: 650; }
  .label { flex: 1; min-width: 0; }
  .icon { display: inline-flex; }
  .icon.home { color: var(--accent); }
  .icon.review { color: var(--spark); }
  .icon.settings { color: var(--text-2); }
  .count {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: 12.5px; font-weight: 600; color: var(--text-2);
  }

  .foot { margin-top: auto; display: grid; gap: 8px; }
  .pill {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 4px 8px; font-size: 12.5px; color: var(--text-2);
  }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--ok); }
  .pill.recording, .pill.meeting { color: var(--text); }
  .pill.recording .dot, .pill.meeting .dot { background: var(--rec); animation: pulse 1.4s ease-in-out infinite; }
  .pill.transcribing .dot { background: var(--accent); }
  @keyframes pulse { 50% { opacity: .35; } }

  /* Labels wrap rather than overflow the sidebar ("Enregistrement en cours"). */
  .new { justify-content: flex-start; width: 100%; background: var(--surface); white-space: normal; text-align: left; line-height: 1.3; padding-block: 6px; }
  .rec { flex: none; width: 12px; height: 12px; border-radius: 50%; background: var(--rec); box-shadow: inset 0 0 0 1px var(--border); }

  .solo { height: 100%; }
  main { position: relative; min-width: 0; min-height: 0; background: var(--bg); }
  .notice {
    position: absolute; left: 50%; bottom: 20px; transform: translateX(-50%);
    pointer-events: none; max-width: calc(100% - 56px);
  }
  .notice span {
    display: block; padding: 8px 16px; border-radius: var(--r-card);
    background: var(--text); color: var(--bg); font-size: 13px;
    box-shadow: var(--shadow-pop);
    animation: rise 200ms var(--ease);
  }
  @keyframes rise { from { opacity: 0; transform: translateY(6px); } }
</style>
