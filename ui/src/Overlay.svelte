<script lang="ts">
  // Dictation pill shown by the backend in its own transparent, never-focused window (#overlay).
  import { onMount } from 'svelte';
  import { api, events, type DictationStatus } from './lib/api';
  import { bindLocale, t, type Lang } from './lib/i18n';
  import Owl from './lib/Owl.svelte';

  const locale = $state<{ lang: Lang }>({ lang: 'fr' });
  bindLocale(locale);

  // Centre bars move most, edges least: reads as a voice, not an equalizer.
  const WEIGHTS = [0.45, 0.7, 0.9, 1, 0.9, 0.7, 0.45];
  let status = $state<DictationStatus>('recording');
  let level = $state(0);
  let jitter = $state(WEIGHTS.map(() => 1));

  onMount(() => {
    api.getSettings().then((s) => (locale.lang = s.ui_language), () => {});
    const offs = [
      events.dictationStatus((s) => (status = s)),
      events.dictationLevel((l) => {
        // Speech RMS sits around 0.02-0.2: sqrt + gain spreads it over the bar height.
        const target = Math.min(1, Math.sqrt(Math.max(0, l)) * 1.8);
        // Fast attack, slower release, like a VU meter.
        level += (target - level) * (target > level ? 0.6 : 0.25);
        jitter = WEIGHTS.map(() => 0.8 + Math.random() * 0.4);
      }),
    ];
    return () => offs.forEach((o) => o.then((f) => f()));
  });

  const scale = (i: number) => Math.min(1, 0.14 + 0.86 * level * WEIGHTS[i] * jitter[i]);
</script>

<div class="pill" class:thinking={status === 'transcribing'}>
  <Owl state={status === 'transcribing' ? 'thinking' : 'listening'} size={28} />
  <div class="wave" style:--level={level.toFixed(2)}>
    {#each WEIGHTS as w, i (i)}
      <span style:--s={scale(i).toFixed(3)} style:--w={w} style:--i={i}></span>
    {/each}
  </div>
  <p class="sr-only" aria-live="polite">{t(status === 'transcribing' ? 'status.transcribing' : 'status.recording')}</p>
</div>

<style>
  :global(html.overlay), :global(html.overlay body) { background: transparent; }
  :global(html.overlay #app) { display: grid; place-items: center; user-select: none; cursor: default; }

  .pill {
    display: flex; align-items: center; gap: 12px;
    height: 44px; padding: 0 16px 0 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 86%, transparent);
    backdrop-filter: blur(20px) saturate(1.6);
    /* Two shadows: a tight one to detach from any wallpaper, a soft one for depth. */
    box-shadow: 0 1px 2px rgb(0 0 0 / .14), 0 4px 10px rgb(0 0 0 / .12);
    animation: appear 180ms var(--ease);
  }
  .wave { display: flex; align-items: center; gap: 3px; height: 24px; }
  .wave span {
    width: 3px; height: 24px; border-radius: 2px;
    background: var(--accent);
    transform: scaleY(var(--s));
    transition: transform 90ms linear;
  }
  /* Transcribing: bars settle low and breathe, one after the other. */
  .thinking .wave span {
    transform: scaleY(.2);
    background: var(--text-2);
    transition-duration: 200ms;
    transition-timing-function: var(--ease);
    animation: breathe 900ms ease-in-out calc(var(--i) * 90ms) infinite;
  }
  @keyframes breathe { 50% { opacity: .35; } }
  @keyframes appear { from { opacity: 0; transform: translateY(6px) scale(.96); } }

  @media (prefers-reduced-motion: reduce) {
    .pill { animation: none; }
    /* Static shape; only the opacity follows the voice level. */
    .wave { opacity: calc(.45 + var(--level) * .55); }
    .wave span, .thinking .wave span { transform: scaleY(calc(.25 + var(--w) * .45)); transition: none; animation: none; }
  }
</style>
