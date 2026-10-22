<script lang="ts" module>
  export type IconName = 'home' | 'review' | 'settings' | 'record' | 'mic' | 'spark' | 'check' | 'chevron';
</script>

<script lang="ts">
  // Hand-drawn 16px icons: 1.75 stroke, round caps, currentColor.

  let { name, size = 16 }: { name: IconName; size?: number } = $props();

  const paths: Record<IconName, string> = {
    home: 'M2.75 7 8 2.75 13.25 7v6.25h-3.5v-3.5h-3.5v3.5h-3.5Z',
    review: 'M3 4h10M3 8h5M3 12h3.5M9.5 11.25l1.75 1.75 3-3.25',
    settings: 'M2.5 4.5h2M7.5 4.5h6M2.5 11.5h6M11.5 11.5h2M7.5 4.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 1 1 3 0M11.5 11.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 1 1 3 0',
    record: 'M12.5 8a4.5 4.5 0 1 1-9 0 4.5 4.5 0 1 1 9 0Z',
    mic: 'M8 2.25a2 2 0 0 1 2 2V7.5a2 2 0 0 1-4 0V4.25a2 2 0 0 1 2-2ZM4 7.5a4 4 0 0 0 8 0M8 11.5v2.25',
    spark: 'M8 .5C8.6 5 11 7.4 15.5 8 11 8.6 8.6 11 8 15.5 7.4 11 5 8.6.5 8 5 7.4 7.4 5 8 .5Z',
    check: 'M3.25 8.5 6.5 11.75 12.75 4.75',
    chevron: 'M6 3.5 10.5 8 6 12.5',
  };
  const filled = $derived(name === 'record' || name === 'spark');
</script>

<svg
  class="icon"
  viewBox="0 0 16 16"
  width={size}
  height={size}
  aria-hidden="true"
  fill={filled ? 'currentColor' : 'none'}
  stroke={filled ? 'none' : 'currentColor'}
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
><path d={paths[name]} /></svg>

<style>
  .icon { display: inline-block; flex: none; }
</style>
