<script lang="ts">
  import { onMount } from 'svelte';
  import { app, go, init, owlState, reviewCount, type Route } from './lib/state.svelte';
  import { t } from './lib/i18n';
  import Owl from './lib/Owl.svelte';
  import ModelSetup from './lib/ModelSetup.svelte';
  import Home from './screens/Home.svelte';
  import NewMeeting from './screens/NewMeeting.svelte';
  import Recording from './screens/Recording.svelte';
  import ReviewQueue from './screens/ReviewQueue.svelte';
  import Review from './screens/Review.svelte';
  import Settings from './screens/Settings.svelte';

  onMount(() => {
    init();
  });

  const nav: { route: Route['name']; key: string; match: Route['name'][] }[] = [
    { route: 'home', key: 'nav.home', match: ['home', 'new', 'recording'] },
    { route: 'queue', key: 'nav.review', match: ['queue', 'review'] },
    { route: 'settings', key: 'nav.settings', match: ['settings'] },
  ];

  const status = $derived(
    app.dictationStatus !== 'idle' ? app.dictationStatus : app.recordingId ? 'meeting' : 'idle',
  );
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="brand" data-tauri-drag-region>Barnaby</div>
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
              <span>{t(item.key)}</span>
              {#if item.route === 'queue' && reviewCount() > 0}
                <span class="count tabular" aria-label={t('nav.review_badge', { n: reviewCount() })}>{reviewCount()}</span>
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
      {#if app.mascot}<Owl state={owlState()} size={64} />{/if}
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
    {#if app.settings}<ModelSetup />{/if}
    <p class="notice" role="status" aria-live="polite">{#if app.notice}<span>{app.notice}</span>{/if}</p>
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 216px 1fr; height: 100%; }

  .sidebar {
    display: flex; flex-direction: column;
    background: var(--sidebar);
    backdrop-filter: blur(30px) saturate(1.8);
    border-right: 1px solid var(--border);
    padding: 0 10px 20px;
    min-height: 0;
  }
  .brand { padding: 22px 10px 18px; font-weight: 600; font-size: 15px; }
  nav ul { display: grid; gap: 2px; }
  .nav-item {
    display: flex; align-items: center; justify-content: space-between; width: 100%;
    padding: 6px 10px; border: 0; border-radius: var(--r-control);
    background: transparent; text-align: left; cursor: pointer; font-size: 14px;
    transition: background-color 150ms var(--ease);
  }
  .nav-item:hover { background: var(--fill); }
  .nav-item.current { background: var(--fill-strong); font-weight: 600; }
  .count {
    min-width: 20px; padding: 0 6px; border-radius: 999px;
    background: var(--warn); color: var(--bg);
    font-size: 11.5px; font-weight: 600; line-height: 18px; text-align: center;
  }

  .foot { margin-top: auto; display: grid; justify-items: center; gap: 12px; }
  .pill {
    display: inline-flex; align-items: center; gap: 7px; justify-self: stretch;
    padding: 4px 10px; border-radius: 999px; background: var(--fill);
    font-size: 12.5px; color: var(--text-2);
  }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--ok); }
  .pill.recording, .pill.meeting { color: var(--text); }
  .pill.recording .dot, .pill.meeting .dot { background: var(--danger); animation: pulse 1.4s ease-in-out infinite; }
  .pill.transcribing .dot { background: var(--accent); }
  @keyframes pulse { 50% { opacity: .35; } }

  main { position: relative; min-width: 0; min-height: 0; }
  .notice {
    position: absolute; left: 50%; bottom: 18px; transform: translateX(-50%);
    pointer-events: none; max-width: calc(100% - 56px);
  }
  .notice span {
    display: block; padding: 8px 14px; border-radius: var(--r-card);
    background: var(--text); color: var(--bg); font-size: 13px;
    box-shadow: 0 8px 24px rgb(0 0 0 / .18);
    animation: rise 200ms var(--ease);
  }
  @keyframes rise { from { opacity: 0; transform: translateY(6px); } }
</style>
