<script lang="ts">
  // The mascot cut out from design/references/owl-sheet.png (ui/src/assets/owl), one pose per system state.
  import type { OwlState } from './state.svelte';
  import { t } from './i18n';

  let { state = 'idle', size = 56 }: { state?: OwlState; size?: number } = $props();

  const art = import.meta.glob<string>('../assets/owl/*.png', { eager: true, import: 'default' });
</script>

<span class="owl {state}" style:--size="{size}px">
  <img src={art[`../assets/owl/${state}.png`]} alt={t('owl.label', { state: t(`owl.${state}`) })} draggable="false" />
</span>

<style>
  .owl {
    display: grid; place-items: center; flex: none;
    width: var(--size); height: var(--size); border-radius: 50%;
    background: var(--owl-field);
  }
  img { width: 80%; height: 80%; object-fit: contain; animation: breathe 5s ease-in-out infinite; transform-origin: 50% 100%; }
  .error img { width: 72%; height: 72%; }
  .success img { animation: hop 240ms var(--ease) 1; }

  @keyframes breathe { 50% { scale: 1.025 1.015; } }
  @keyframes hop { 50% { translate: 0 -6%; } }

  @media (prefers-reduced-motion: reduce) {
    img { animation: none; }
  }
</style>
