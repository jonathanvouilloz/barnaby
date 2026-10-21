<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api, events, type ModelInfo, type Settings } from '../lib/api';
  import { app, applySettings, attempt, notify, setMascot } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatSize } from '../lib/format';
  import { isLoopbackUrl } from '../lib/settings';
  import ShortcutInput from '../lib/ShortcutInput.svelte';

  // Explicit save: shortcuts are re-registered by the backend and may be refused as a whole.
  let draft = $state<Settings>($state.snapshot(app.settings!));
  let saving = $state(false);
  let error = $state('');
  let models = $state<ModelInfo[]>([]);
  let progress = $state<Record<string, number>>({});
  let hasSecret = $state(false);
  let secret = $state('');
  let errorEl = $state<HTMLElement>();

  const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(app.settings));
  const whisper = $derived(models.filter((m) => m.kind === 'whisper'));
  const llmNotLocal = $derived(draft.llm.enabled && draft.processing_mode === 'local' && !isLoopbackUrl(draft.llm.base_url));

  async function loadModels() {
    models = (await attempt(api.listModels)) ?? models;
  }

  onMount(() => {
    loadModels();
    attempt(api.hasWebhookSecret).then((v) => (hasSecret = v ?? false));
    const off = events.modelProgress(({ id, progress: p }) => {
      progress[id] = p;
      if (p >= 1) {
        delete progress[id];
        loadModels();
      }
    });
    return () => off.then((f) => f());
  });

  async function save() {
    saving = true;
    error = '';
    const next = $state.snapshot(draft);
    // An emptied number field binds to null; the backend expects whole milliseconds.
    const ms = (v: number) => Math.max(0, Math.round(Number(v) || 0));
    next.dictation.paste_delay_before_ms = ms(next.dictation.paste_delay_before_ms);
    next.dictation.paste_delay_after_ms = ms(next.dictation.paste_delay_after_ms);
    try {
      await api.saveSettings(next);
      applySettings(next);
      notify(t('settings.saved'));
    } catch (e) {
      error = String(e);
      await tick();
      errorEl?.scrollIntoView({ block: 'nearest' });
    }
    saving = false;
  }

  async function download(m: ModelInfo) {
    progress[m.id] = 0;
    if ((await attempt(() => api.downloadModel(m.id))) === undefined) delete progress[m.id];
  }

  async function removeModel(m: ModelInfo) {
    if ((await attempt(() => api.deleteModel(m.id))) !== undefined) loadModels();
  }

  async function storeSecret(value: string) {
    if ((await attempt(() => api.setWebhookSecret(value))) === undefined) return;
    hasSecret = value !== '';
    secret = '';
    notify(t(value ? 'settings.secret_saved' : 'settings.secret_cleared'));
  }

  const size = (mb: number) => {
    const s = formatSize(mb, app.lang);
    return t(`settings.size_${s.unit}`, { n: s.n });
  };
  const modelLabel = (m: ModelInfo) => (m.installed ? m.name : t('settings.not_installed', { name: m.name }));
</script>

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <h1>{t('settings.title')}</h1>
    {#if dirty}<span class="secondary">{t('settings.unsaved')}</span>{/if}
    <button class="btn primary" disabled={!dirty || saving} onclick={save}>{t('common.save')}</button>
  </header>

  <div class="screen-body">
    {#if error}<p class="error" role="alert" bind:this={errorEl}>{error}</p>{/if}

    <div class="sections">
      <section class="card" aria-labelledby="s-dictation">
        <h2 id="s-dictation">{t('settings.dictation')}</h2>
        <fieldset>
          <legend class="secondary">{t('settings.mode')}</legend>
          {#each ['hold', 'toggle'] as const as mode (mode)}
            <label class="choice">
              <input type="radio" name="dictation-mode" value={mode} bind:group={draft.dictation.mode} />
              <span><span class="strong">{t(`settings.${mode}`)}</span> <span class="secondary">{t(`settings.${mode}_hint`)}</span></span>
            </label>
          {/each}
        </fieldset>
        <label class="field"><span>{t('settings.model')}</span>
          <select bind:value={draft.dictation.model}>
            {#each whisper as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
          </select>
        </label>
        <div class="two">
          <label class="field"><span>{t('settings.delay_before')}</span><input type="number" min="0" max="2000" step="10" bind:value={draft.dictation.paste_delay_before_ms} /></label>
          <label class="field"><span>{t('settings.delay_after')}</span><input type="number" min="0" max="5000" step="50" bind:value={draft.dictation.paste_delay_after_ms} /></label>
        </div>
        <label class="check"><input type="checkbox" bind:checked={draft.dictation.keep_history} /> {t('settings.keep_history')}</label>
      </section>

      <section class="card" aria-labelledby="s-meeting">
        <h2 id="s-meeting">{t('settings.meeting')}</h2>
        <label class="field"><span>{t('settings.model')}</span>
          <select bind:value={draft.meeting.model}>
            {#each whisper as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
          </select>
        </label>
        <label class="field">
          <span>{t('settings.threshold')} <output class="tabular">{new Intl.NumberFormat(app.lang, { minimumFractionDigits: 2 }).format(draft.meeting.confidence_threshold)}</output></span>
          <input type="range" min="0.3" max="0.9" step="0.05" bind:value={draft.meeting.confidence_threshold} aria-describedby="threshold-hint" />
          <span id="threshold-hint" class="secondary">{t('settings.threshold_hint')}</span>
        </label>
        <label class="field"><span>{t('settings.retention')}</span>
          <select bind:value={draft.meeting.retention}>
            {#each ['on_review', 'seven_days', 'forever'] as r (r)}<option value={r}>{t(`settings.retention.${r}`)}</option>{/each}
          </select>
        </label>
      </section>

      <section class="card" aria-labelledby="s-models">
        <h2 id="s-models">{t('settings.models')}</h2>
        <ul class="models">
          {#each models as m (m.id)}
            {@const p = progress[m.id]}
            <li>
              <div class="grow">
                <span class="strong">{m.name}</span>
                <span class="secondary">{t(`settings.kind.${m.kind}`)} · <span class="tabular">{size(m.size_mb)}</span></span>
                {#if p !== undefined}
                  <progress max="1" value={p} aria-label={t('settings.downloading', { pct: Math.round(p * 100) })}></progress>
                {/if}
              </div>
              {#if p !== undefined}
                <span class="secondary tabular" aria-live="polite">{t('settings.downloading', { pct: Math.round(p * 100) })}</span>
              {:else if m.installed}
                <span class="installed">{t('settings.installed')}</span>
                <button class="btn ghost small danger" aria-label={t('settings.delete_name', { name: m.name })} onclick={() => removeModel(m)}>{t('common.delete')}</button>
              {:else}
                <button class="btn small" aria-label={t('settings.download_name', { name: m.name })} onclick={() => download(m)}>{t('settings.download')}</button>
              {/if}
            </li>
          {/each}
        </ul>
      </section>

      <section class="card" aria-labelledby="s-shortcuts">
        <h2 id="s-shortcuts">{t('settings.shortcuts')}</h2>
        <p class="secondary">{t('settings.sc.hint')}</p>
        {#each ['dictation', 'meeting_toggle', 'marker', 'review_queue'] as const as k (k)}
          <ShortcutInput label={t(`settings.sc.${k}`)} bind:value={draft.shortcuts[k]} />
        {/each}
      </section>

      <section class="card" aria-labelledby="s-dest">
        <h2 id="s-dest">{t('settings.destinations')}</h2>
        <label class="check"><input type="checkbox" bind:checked={draft.destinations.folder.enabled} /> {t('settings.folder_enabled')}</label>
        <label class="field"><span>{t('settings.folder_path')}</span><input type="text" bind:value={draft.destinations.folder.path} disabled={!draft.destinations.folder.enabled} /></label>
        <hr />
        <label class="check"><input type="checkbox" bind:checked={draft.destinations.webhook.enabled} /> {t('settings.webhook_enabled')}</label>
        <label class="field"><span>{t('settings.webhook_url')}</span><input type="url" bind:value={draft.destinations.webhook.url} placeholder="https://" disabled={!draft.destinations.webhook.enabled} /></label>
        <form class="secret" onsubmit={(e) => { e.preventDefault(); if (secret) storeSecret(secret); }}>
          <label class="field grow"><span>{t('settings.secret')}</span>
            <input type="password" bind:value={secret} autocomplete="off" aria-describedby="secret-state" />
          </label>
          <button class="btn" type="submit" disabled={!secret}>{t('settings.secret_save')}</button>
        </form>
        <p class="secondary row" id="secret-state">
          {t(hasSecret ? 'settings.secret_set' : 'settings.secret_none')}
          {#if hasSecret}<button class="btn ghost small danger" onclick={() => storeSecret('')}>{t('settings.secret_clear')}</button>{/if}
        </p>
      </section>

      <section class="card" aria-labelledby="s-llm">
        <h2 id="s-llm">{t('settings.llm')}</h2>
        <label class="check"><input type="checkbox" bind:checked={draft.llm.enabled} /> {t('settings.llm_enabled')}</label>
        <div class="two">
          <label class="field"><span>{t('settings.base_url')}</span><input type="url" bind:value={draft.llm.base_url} disabled={!draft.llm.enabled} /></label>
          <label class="field"><span>{t('settings.llm_model')}</span><input type="text" bind:value={draft.llm.model} disabled={!draft.llm.enabled} /></label>
        </div>
        <p class="secondary">{t('settings.llm_local_note')}</p>
        {#if llmNotLocal}<p class="warn" role="status">{t('settings.llm_not_local')}</p>{/if}
      </section>

      <section class="card" aria-labelledby="s-lang">
        <h2 id="s-lang">{t('settings.language')}</h2>
        <div class="two">
          <label class="field"><span>{t('settings.ui_language')}</span>
            <select bind:value={draft.ui_language}>
              <option value="fr">Français</option>
              <option value="en">English</option>
            </select>
          </label>
          <label class="field"><span>{t('settings.transcription_language')}</span>
            <select bind:value={draft.transcription_language}>
              {#each ['auto', 'fr', 'en'] as l (l)}<option value={l}>{t(`settings.lang.${l}`)}</option>{/each}
            </select>
          </label>
        </div>
      </section>

      <section class="card" aria-labelledby="s-misc">
        <h2 id="s-misc">{t('settings.misc')}</h2>
        <label class="check"><input type="checkbox" bind:checked={draft.notification_sound} /> {t('settings.notification_sound')}</label>
        <label class="check">
          <input type="checkbox" checked={app.mascot} onchange={(e) => setMascot(e.currentTarget.checked)} aria-describedby="mascot-hint" />
          {t('settings.mascot')}
        </label>
        <p class="secondary" id="mascot-hint">{t('settings.mascot_hint')}</p>
      </section>
    </div>
  </div>
</section>

<style>
  .sections { display: grid; gap: 16px; max-width: 680px; }
  .card { padding: 16px 18px; display: grid; gap: 12px; }
  fieldset { border: 0; margin: 0; padding: 0; display: grid; gap: 6px; }
  legend { padding: 0; margin-bottom: 4px; }
  .choice { display: flex; gap: 8px; align-items: baseline; }
  .check { display: flex; gap: 8px; align-items: center; }
  .strong { font-weight: 500; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .grow { flex: 1; min-width: 0; }
  hr { border: 0; border-top: 1px solid var(--border); margin: 2px 0; width: 100%; }
  output { color: var(--text); font-weight: 500; margin-left: 4px; }
  .models { display: grid; }
  .models li { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-top: 1px solid var(--border); }
  .models li:first-child { border-top: 0; }
  .models .grow { display: grid; }
  progress { width: 100%; height: 4px; margin-top: 4px; accent-color: var(--accent); }
  .installed { font-size: 12.5px; color: var(--ok); }
  .secret { display: flex; gap: 8px; align-items: end; }
  .row { display: flex; align-items: center; gap: 8px; }
  .warn { color: var(--warn); font-size: 13px; }
  .error {
    max-width: 680px; margin-bottom: 16px; padding: 10px 14px; border-radius: var(--r-card);
    color: var(--danger); border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    background: color-mix(in srgb, var(--danger) 8%, var(--surface));
  }
</style>
