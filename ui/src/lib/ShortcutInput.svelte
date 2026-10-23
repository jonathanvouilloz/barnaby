<script lang="ts">
  import { acceleratorFromKey, keyLabel, shortcutWarning } from './accelerator';
  import { api } from './api';
  import { t } from './i18n';
  import { app } from './state.svelte';

  let { value = $bindable(), label }: { value: string; label: string } = $props();

  let capturing = $state(false);
  let hint = $state('');
  const warning = $derived(shortcutWarning(value));

  // Barnaby's own shortcuts are off while capturing, or the current one never reaches this button.
  function start() {
    capturing = true;
    api.suspendShortcuts().catch(() => {});
  }

  async function onkeydown(e: KeyboardEvent) {
    if (!capturing) return;
    if (e.key === 'Tab') return;
    e.preventDefault();
    if (e.key === 'Escape') return stop();
    const accel = acceleratorFromKey(e);
    if (accel === 'need_modifier') hint = t('settings.sc.need_modifier');
    else if (accel) {
      if (await api.checkShortcut(accel).catch(() => false)) {
        hint = t('settings.sc.taken', { keys: keyLabel(accel, app.lang) });
        return;
      }
      value = accel;
      stop();
    }
  }

  function stop() {
    if (!capturing) return;
    capturing = false;
    hint = '';
    api.resumeShortcuts().catch(() => {});
  }
</script>

<div class="row">
  <span class="label">{label}</span>
  <button
    type="button"
    class="btn keys"
    class:capturing
    aria-label="{label}, {capturing ? t('settings.sc.press') : keyLabel(value, app.lang)}"
    onclick={() => (capturing ? stop() : start())}
    {onkeydown}
    onblur={stop}
  >
    {#if capturing}{t('settings.sc.press')}{:else}<kbd>{keyLabel(value, app.lang)}</kbd>{/if}
  </button>
</div>
<!-- The button's own label is not re-read when capture starts: this line says it. -->
<span class="sr-only" aria-live="polite">{capturing ? t('settings.sc.press') : ''}</span>
{#if hint}<p class="secondary hint" role="status">{hint}</p>
{:else if warning && !capturing}<p class="secondary hint" role="status">{t(`settings.sc.warn_${warning}`, { keys: keyLabel(value, app.lang) })}</p>{/if}

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .keys { min-width: 170px; }
  .capturing { border-color: var(--accent); color: var(--accent-text); }
  kbd { font: inherit; font-variant-numeric: tabular-nums; }
  .hint { text-align: right; color: var(--warn); }
</style>
