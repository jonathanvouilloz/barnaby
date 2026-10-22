<script lang="ts">
  // The real dictation, pasted into this window: the shortcut is saved for good before the try.
  import { onMount, tick } from 'svelte';
  import { api, events } from '../lib/api';
  import { t } from '../lib/i18n';
  import { app, applySettings, errorText } from '../lib/state.svelte';
  import Owl from '../lib/Owl.svelte';
  import ShortcutInput from '../lib/ShortcutInput.svelte';

  let { ready = $bindable() }: { ready: boolean } = $props();
  ready = false;

  let shortcut = $state(app.settings!.shortcuts.dictation);
  let error = $state('');
  let text = $state('');
  let worked = $state(false);
  let area = $state<HTMLTextAreaElement>();
  // Text before the current dictation: success means the dictation really changed the box.
  let before = '';

  function check() {
    if (text.trim() && text !== before) worked = ready = true;
  }

  function retry() {
    text = before = '';
    worked = ready = false;
    area?.focus();
  }

  // A refused shortcut (conflict) keeps the step open with the backend's reason.
  async function save(value: string) {
    shortcut = value;
    error = '';
    const s = $state.snapshot(app.settings!);
    const next = { ...s, shortcuts: { ...s.shortcuts, dictation: value } };
    try {
      await api.saveSettings(next);
      applySettings(next);
      area?.focus();
    } catch (e) {
      error = errorText(String(e));
      shortcut = app.settings!.shortcuts.dictation;
    }
  }

  // Dictation pastes; typing by hand does not count.
  function oninput(e: Event) {
    if ((e as InputEvent).inputType === 'insertFromPaste') check();
  }

  onMount(() => {
    area?.focus();
    const offs = [
      events.dictationStatus((s) => s === 'recording' && (before = text)),
      // The paste may land just after the event: check the box, never trust the event alone.
      events.dictationDone(() => tick().then(check)),
    ];
    return () => offs.forEach((o) => o.then((f) => f()));
  });
</script>

<div class="shortcut">
  <ShortcutInput label={t('onboarding.dictation.shortcut')} bind:value={() => shortcut, save} />
</div>
{#if error}<p class="problem" role="alert">{error}</p>{/if}

<label class="field try">
  <span>{t(app.settings!.dictation.mode === 'hold' ? 'onboarding.dictation.hint_hold' : 'onboarding.dictation.hint_toggle')}</span>
  <textarea bind:this={area} bind:value={text} {oninput} rows="4" placeholder={t('onboarding.dictation.placeholder')}></textarea>
</label>

<div class="result" aria-live="polite">
  {#if worked}
    {#if app.mascot}<Owl state="success" size={56} />{/if}
    <p>{t('onboarding.dictation.worked')}</p>
    <button class="btn ghost" onclick={retry}>{t('common.retry')}</button>
  {/if}
</div>

<style>
  .shortcut { width: 100%; }
  .try { width: 100%; }
  .try > span { font-size: 14px; color: var(--text); font-weight: 400; }
  textarea { margin-top: 4px; resize: none; }
  .result { display: flex; align-items: center; gap: 12px; min-height: 56px; }
  .result p { font-weight: 550; }
  .problem { color: var(--danger); }
</style>
