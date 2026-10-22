<script lang="ts">
  // The app's signature: marks what awaits the user, and bursts away when it is settled.
  let { size = 12, active = true, label }: { size?: number; active?: boolean; label?: string } = $props();

  let leaving = $state(false);
  let was = false;
  $effect.pre(() => {
    if (was && !active && !matchMedia('(prefers-reduced-motion: reduce)').matches) leaving = true;
    if (active) leaving = false;
    was = active;
  });
</script>

{#if active || leaving}
  <svg
    class="spark"
    class:leaving
    viewBox="0 0 16 16"
    width={size}
    height={size}
    role={label ? 'img' : undefined}
    aria-label={label}
    aria-hidden={label ? undefined : 'true'}
    onanimationend={() => (leaving = false)}
  >
    <path d="M8 .5C8.6 5 11 7.4 15.5 8 11 8.6 8.6 11 8 15.5 7.4 11 5 8.6.5 8 5 7.4 7.4 5 8 .5Z" />
  </svg>
{/if}

<style>
  .spark { display: inline-block; flex: none; fill: var(--spark); overflow: visible; }
  .leaving { animation: settle 240ms var(--ease) forwards; transform-origin: center; }
  @keyframes settle {
    45% { transform: rotate(40deg) scale(1.45); opacity: 1; }
    100% { transform: rotate(90deg) scale(.3); opacity: 0; }
  }
</style>
