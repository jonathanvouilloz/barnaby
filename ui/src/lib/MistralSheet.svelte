<script lang="ts">
  // What leaves the machine with Mistral transcription, and the key that turns it on.
  import { api } from './api';
  import { t } from './i18n';
  import { attempt, notify } from './state.svelte';
  import { keyErrorKey } from './settings';
  import Sheet from './Sheet.svelte';

  // hasKey: a key is already saved, so the sheet only discloses and confirms.
  let { open, hasKey = false, oncancel, onconfirm }: { open: boolean; hasKey?: boolean; oncancel: () => void; onconfirm: () => void } = $props();

  let key = $state('');
  let checking = $state(false);
  let error = $state('');

  function cancel() {
    key = '';
    error = '';
    oncancel();
  }

  async function confirm() {
    if (hasKey) return onconfirm();
    // A refused key is never saved: the first meeting would fail with it.
    checking = true;
    error = '';
    let unchecked = false;
    try {
      await api.testMistralKey(key.trim());
    } catch (e) {
      // Offline is not a wrong key: save it, and say it was not checked.
      unchecked = String(e) === 'mistral_unreachable';
      if (!unchecked) {
        error = t(keyErrorKey(String(e)));
        return;
      }
    } finally {
      checking = false;
    }
    if ((await attempt(() => api.setMistralApiKey(key.trim()))) === undefined) return;
    key = '';
    notify(t(unchecked ? 'mistral.key_saved_unchecked' : 'settings.key_saved'));
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
      <input type="password" bind:value={key} autocomplete="off" aria-invalid={error ? 'true' : undefined} aria-describedby={error ? 'mistral-key-error' : undefined} oninput={() => (error = '')} />
    </label>
    {#if error}<p id="mistral-key-error" class="error" role="alert">{error}</p>{/if}
  {/if}
  {#snippet actions()}
    <button class="btn" onclick={cancel}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={(!hasKey && !key.trim()) || checking} onclick={confirm}>{t(checking ? 'mistral.checking' : hasKey ? 'mistral.use_saved' : 'mistral.confirm')}</button>
  {/snippet}
</Sheet>

<style>
  .bullets { list-style: disc; padding-left: 20px; display: grid; gap: 4px; }
  .error { font-size: 12.5px; color: var(--danger); }
</style>
