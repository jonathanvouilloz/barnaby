<script lang="ts">
  // Provisional SVG owl after design/references/owl-sheet.png, until the cut-out mascot lands.
  // One expression per system state; colours come from the --owl-* tokens only.
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
  <path class="spark" d="M32 0.5C32.4 3.4 33.6 4.6 36.5 5 33.6 5.4 32.4 6.6 32 9.5 31.6 6.6 30.4 5.4 27.5 5 30.4 4.6 31.6 3.4 32 0.5Z" />
  <g class="bird">
    <path class="body line" d="M15 45C15 36 22 33 32 33S49 36 49 45C49 54 43 59 32 59S15 54 15 45Z" />
    <path class="belly" d="M22 48C22 42 26 40 32 40S42 42 42 48C42 54 38 57 32 57S22 54 22 48Z" />
    <path class="wing line" d="M16.5 41C12.5 45 13 52 18.5 55 19.5 50 19.5 45 16.5 41Z" />
    <path class="wing line" d="M47.5 41C51.5 45 51 52 45.5 55 44.5 50 44.5 45 47.5 41Z" />
    <ellipse class="foot line" cx="26.5" cy="59" rx="3.2" ry="2.2" />
    <ellipse class="foot line" cx="37.5" cy="59" rx="3.2" ry="2.2" />

    <g class="head">
      <path
        class="skull line"
        d="M13 24C11 19 11 15 12.5 11.5 15.5 13 18.5 15 21 16.2 24.5 14.6 28 14 32 14S39.5 14.6 43 16.2C45.5 15 48.5 13 51.5 11.5 53 15 53 19 51 24 54 29 54 35 51 39 47 43.5 40 44.5 32 44.5S17 43.5 13 39C10 35 10 29 13 24Z"
      />
      <path class="face" d="M32 21C29 18 24 18 20 20 15 23 14 30 16 35 18 39 24 41.5 32 41.5S46 39 48 35C50 30 49 23 44 20 40 18 35 18 32 21Z" />

      <g class="look">
        <g class="eyes">
          <circle class="eye" cx="24" cy="29" r="5.2" />
          <circle class="eye" cx="40" cy="29" r="5.2" />
          <circle class="glint" cx="25.8" cy="27.2" r="1.7" />
          <circle class="glint" cx="41.8" cy="27.2" r="1.7" />
          <circle class="glint" cx="22.6" cy="31" r=".8" />
          <circle class="glint" cx="38.6" cy="31" r=".8" />
        </g>
      </g>
      <path class="happy" d="M20 30.5Q24 25.5 28 30.5M36 30.5Q40 25.5 44 30.5" />

      <path class="beak line" d="M29.6 32.4H34.4L32 36.6Z" />
      <ellipse class="mouth" cx="32" cy="39" rx="1.5" ry="1.8" />
    </g>
  </g>
</svg>

<style>
  .owl { display: block; overflow: visible; }
  .line { stroke: var(--owl-line); stroke-width: 1.6; stroke-linejoin: round; }
  .skull { stroke-width: 2; }
  .body, .skull { fill: var(--owl-body); }
  .wing { fill: var(--owl-shade); }
  .belly { fill: var(--owl-belly); }
  .face { fill: var(--owl-face); }
  .eye { fill: var(--owl-eye); }
  .glint { fill: var(--owl-face); }
  .beak, .foot { fill: var(--owl-beak); }
  .beak { stroke-width: 1.2; }
  .happy { fill: none; stroke: var(--owl-eye); stroke-width: 2.2; stroke-linecap: round; opacity: 0; }
  .mouth { fill: var(--owl-line); opacity: 0; }
  .spark { fill: var(--spark); opacity: 0; transform-box: fill-box; transform-origin: center; transform: scale(.4); }

  .bird, .head, .look, .eyes, .happy, .mouth, .spark {
    transition: transform 200ms var(--ease), opacity 200ms var(--ease);
  }
  .bird { transform-box: view-box; transform-origin: 32px 60px; animation: sway 6s ease-in-out infinite; }
  .head { transform-box: view-box; transform-origin: 32px 40px; }
  .eyes { transform-box: fill-box; transform-origin: center; animation: blink 5s infinite; }

  /* listening: eyes wide, the sparkle shows */
  .listening .eyes { transform: scale(1.08); }
  .listening .spark, .success .spark { opacity: 1; transform: none; }

  /* thinking: looking up */
  .thinking .look { transform: translate(1px, -2px); }

  /* review: head tilted, waiting */
  .review .head { transform: rotate(-10deg); }

  /* success: eyes squeezed with joy, one short hop */
  .success .eyes { opacity: 0; animation: none; }
  .success .happy { opacity: 1; }
  .success .bird { animation: hop 240ms var(--ease) 1; }

  /* error: small pupils, round mouth */
  .error .eyes { transform: scale(.82); }
  .error .mouth { opacity: 1; }

  @keyframes blink {
    0%, 95%, 100% { scale: 1 1; }
    97.5% { scale: 1 .1; }
  }
  @keyframes sway {
    0%, 100% { rotate: 0deg; }
    50% { rotate: 1.5deg; }
  }
  @keyframes hop {
    50% { translate: 0 -4px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .bird, .head, .look, .eyes, .happy, .mouth, .spark { transition: none; animation: none; }
  }
</style>
