<script lang="ts">
  // Three moments of a meeting, each drawn in lines with the ✦ where Barnaby helps.
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';

  const panels = [
    { id: 'notes', key: 'marker' },
    { id: 'review', key: 'review_queue' },
    { id: 'send', key: 'meeting_toggle' },
  ] as const;

  const spark = 'M8 .5C8.6 5 11 7.4 15.5 8 11 8.6 8.6 11 8 15.5 7.4 11 5 8.6.5 8 5 7.4 7.4 5 8 .5Z';
</script>

<ul class="panels">
  {#each panels as p (p.id)}
    <li>
      <svg viewBox="0 0 120 64" aria-hidden="true">
        {#if p.id === 'notes'}
          <path class="ink" d="M8 32h6M18 24v16M24 18v28M30 26v12M36 22v20M42 30v4M48 20v24M54 27v10M60 32h6M70 25v14M76 19v26M82 28v8M88 22v20M94 30v4M100 32h12" />
          <path class="ink soft" d="M48 52h20M48 58h12" />
          <path class="spark" transform="translate(40 2)" d={spark} />
        {:else if p.id === 'review'}
          <path class="ink" d="M10 12h76M10 26h48M10 54h64" />
          <path class="ink dashed" d="M10 40h58" />
          <path class="spark" transform="translate(76 32)" d={spark} />
        {:else}
          <rect class="ink" x="12" y="6" width="40" height="52" rx="4" />
          <path class="ink" d="M20 18h24M20 28h24M20 38h14" />
          <path class="ink" d="M62 32h30M84 24l8 8-8 8" />
          <path class="spark" transform="translate(98 24)" d={spark} />
        {/if}
      </svg>
      <h2>{t(`onboarding.tour.${p.id}`)}</h2>
      <p class="secondary">{t(`onboarding.tour.${p.id}_text`)}</p>
      <p class="key secondary">{t(`settings.sc.${p.key}`)}<kbd>{app.settings!.shortcuts[p.key]}</kbd></p>
    </li>
  {/each}
</ul>

<style>
  .panels { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 32px; width: 100%; }
  li { display: grid; gap: 8px; align-content: start; }
  svg { width: 120px; height: 64px; margin-bottom: 8px; color: var(--text-2); overflow: visible; }
  .ink { fill: none; stroke: currentColor; stroke-width: 1.75; stroke-linecap: round; stroke-linejoin: round; }
  .soft { opacity: .55; }
  .dashed { stroke: var(--spark); stroke-dasharray: 3 5; }
  .spark { fill: var(--spark); }
  .key { display: grid; justify-items: start; gap: 4px; margin-top: 4px; }
  kbd {
    font: inherit; font-weight: 550; color: var(--text); font-variant-numeric: tabular-nums;
    padding: 0 8px; border-radius: var(--r-control); background: var(--fill-strong);
  }
</style>
