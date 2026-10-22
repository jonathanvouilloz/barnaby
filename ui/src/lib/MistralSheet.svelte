<script lang="ts">
  // What leaves the machine with Mistral transcription, and the key that turns it on.
  import { api } from './api';
  import { t } from './i18n';
  import { attempt, notify } from './state.svelte';
  import Sheet from './Sheet.svelte';

  // hasKey: a key is already saved, so the sheet only discloses and confirms.
  let { open, hasKey = false, oncancel, onconfirm }: { open: boolean; hasKey?: boolean; oncancel: () => void; onconfirm: () => void } = $props();

  let key = $state('');

  function cancel() {
    key = '';
    oncancel();
  }

  async function confirm() {
    if (hasKey) return onconfirm();
    if ((await attempt(() => api.setMistralApiKey(key.trim()))) === undefined) return;
    key = '';
    notify(t('settings.key_saved'));
    onconfirm();
  }
</script>

<Sheet {open} title={t('mistral.title')} onclose={cancel}>
  <p>{t('mistral.intro')}</p>
  <ul class="bullets">
    <li>{t('mistral.sent_audio')}</li>
    <li>{t('mistral.sent_vocabulary')}</li>
  </ul>
  <p>{t('mistral.price')}</p>
  <p class="secondary">{t('mistral.terms')}</p>
  {#if !hasKey}
    <label class="field"><span>{t('mistral.key')}</span>
      <input type="password" bind:value={key} autocomplete="off" />
    </label>
  {/if}
  {#snippet actions()}
    <button class="btn" onclick={cancel}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!hasKey && !key.trim()} onclick={confirm}>{t(hasKey ? 'mistral.use_saved' : 'mistral.confirm')}</button>
  {/snippet}
</Sheet>

<style>
  .bullets { list-style: disc; padding-left: 20px; display: grid; gap: 4px; }
</style>
