<script lang="ts">
  // Placeholder owl until the author's mascot lands. One look per system state (spec 10.4).
  import type { OwlState } from './state.svelte';
  import { t } from './i18n';

  let { state = 'idle', size = 56 }: { state?: OwlState; size?: number } = $props();
</script>

<svg
  class="owl {state}"
  viewBox="0 0 64 64"
  width={size}
  height={size}
  role="img"
  aria-label={t('owl.label', { state: t(`owl.${state}`) })}
>
  <g class="bird">
    <g class="head">
      <path class="tuft" d="M14 22 L17 7 L26 16 Z" />
      <path class="tuft right" d="M50 22 L47 7 L38 16 Z" />
      <ellipse class="body" cx="32" cy="37" rx="20" ry="22" />
      <ellipse class="belly" cx="32" cy="48" rx="11" ry="8.5" />
      <circle class="disc" cx="24" cy="30" r="8.5" />
      <circle class="disc" cx="40" cy="30" r="8.5" />
      <g class="pupils">
        <circle class="pupil" cx="24" cy="30" r="4" />
        <circle class="pupil" cx="40" cy="30" r="4" />
        <circle class="glint" cx="25.5" cy="28.5" r="1.3" />
        <circle class="glint" cx="41.5" cy="28.5" r="1.3" />
      </g>
      <path class="lid" d="M15.5 30 A8.5 8.5 0 0 1 32.5 30 Z" />
      <path class="lid" d="M31.5 30 A8.5 8.5 0 0 1 48.5 30 Z" />
      <path class="beak" d="M29 37 L35 37 L32 42 Z" />
    </g>
    <path class="feet" d="M26 58.5 v2.5 M29 58.5 v2.5 M35 58.5 v2.5 M38 58.5 v2.5" />
  </g>
</svg>

<style>
  .owl { display: block; overflow: visible; }
  .body, .tuft { fill: var(--owl-body); }
  .belly { fill: var(--owl-face); opacity: .55; }
  .disc { fill: var(--owl-face); }
  .pupil { fill: #1d1d1f; }
  .glint { fill: #fff; }
  .lid { fill: var(--owl-body); transform-box: fill-box; transform-origin: top; transform: scaleY(0); }
  .beak { fill: var(--owl-beak); }
  .feet { stroke: var(--owl-beak); stroke-width: 1.6; stroke-linecap: round; }

  .bird, .head, .tuft, .pupils, .lid {
    transition: transform 200ms var(--ease);
  }
  .head, .bird { transform-box: view-box; transform-origin: 32px 40px; }
  .tuft { transform-box: fill-box; transform-origin: bottom center; }

  /* listening: tufts up, eyes wide open */
  .listening .tuft { transform: translateY(-1.5px) rotate(-6deg); }
  .listening .tuft.right { transform: translateY(-1.5px) rotate(6deg); }

  /* thinking: eyes half shut, looking up and aside */
  .thinking .lid { transform: scaleY(.55); }
  .thinking .pupils { transform: translate(-1.5px, -1.5px); }

  /* review: head tilted, waiting */
  .review .head { transform: rotate(-10deg); }

  /* success: one short hop, eyes smiling */
  .success .bird { animation: hop 420ms var(--ease) 1; }
  .success .lid { transform: scaleY(.8); }

  /* error: tufts flattened, pupils small */
  .error .tuft { transform: rotate(22deg); }
  .error .tuft.right { transform: rotate(-22deg); }
  .error .pupil { r: 2.6; }

  @keyframes hop {
    40% { transform: translateY(-5px); }
  }

  @media (prefers-reduced-motion: reduce) {
    .bird, .head, .tuft, .pupils, .lid { transition: none; animation: none; }
  }
</style>
