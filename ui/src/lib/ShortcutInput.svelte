<script lang="ts">
  import { acceleratorFromKey } from './accelerator';
  import { t } from './i18n';

  let { value = $bindable(), label }: { value: string; label: string } = $props();

  let capturing = $state(false);
  let hint = $state('');

  function onkeydown(e: KeyboardEvent) {
    if (!capturing) return;
    if (e.key === 'Tab') return;
    e.preventDefault();
    if (e.key === 'Escape') return stop();
    const accel = acceleratorFromKey(e);
    if (accel === 'need_modifier') hint = t('settings.sc.need_modifier');
    else if (accel) {
      value = accel;
      stop();
    }
  }

  function stop() {
    capturing = false;
    hint = '';
  }
</script>

<div class="row">
  <span class="label">{label}</span>
  <button
    type="button"
    class="btn keys"
    class:capturing
    aria-label="{label}, {capturing ? t('settings.sc.press') : value}"
    onclick={() => (capturing = !capturing)}
    {onkeydown}
    onblur={stop}
  >
    {#if capturing}{t('settings.sc.press')}{:else}<kbd>{value}</kbd>{/if}
  </button>
</div>
{#if hint}<p class="secondary hint" role="status">{hint}</p>{/if}

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .keys { min-width: 170px; }
  .capturing { border-color: var(--accent); color: var(--accent); }
  kbd { font: inherit; font-variant-numeric: tabular-nums; }
  .hint { text-align: right; color: var(--warn); }
</style>
