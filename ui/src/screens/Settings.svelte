<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api, events, type ModelInfo, type OutputDevice, type Settings } from '../lib/api';
  import { app, applySettings, attempt, go, notify, setMascot } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatSize } from '../lib/format';
  import { addTerm, cloudSummary, isLoopbackUrl, MISTRAL_MODEL, modeFor, OLLAMA_URL, OPENROUTER_URL } from '../lib/settings';
  import ShortcutInput from '../lib/ShortcutInput.svelte';
  import Sheet from '../lib/Sheet.svelte';
  import MistralSheet from '../lib/MistralSheet.svelte';

  // Explicit save: shortcuts are re-registered by the backend and may be refused as a whole.
  let draft = $state<Settings>($state.snapshot(app.settings!));
  let saving = $state(false);
  let error = $state('');
  let models = $state<ModelInfo[]>([]);
  let progress = $state<Record<string, number>>({});
  let hasSecret = $state(false);
  let secret = $state('');
  let errorEl = $state<HTMLElement>();
  let term = $state('');
  let keys = $state({ llm: '', mistral: '' });
  let hasKeys = $state({ llm: false, mistral: false });
  let disclosureOpen = $state(false);
  let mistralOpen = $state(false);
  let mistralFallback = ''; // meeting model restored when the Mistral sheet is cancelled
  let outputs = $state<OutputDevice[]>([]);

  const CLOUD_MODELS = ['qwen/qwen3-235b-a22b-2507', 'deepseek/deepseek-chat-v3.1', 'z-ai/glm-4.6'];

  const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(app.settings));
  const whisper = $derived(models.filter((m) => m.kind === 'whisper'));
  const parakeet = $derived(models.filter((m) => m.kind === 'parakeet'));
  // Not derived from the URL: typing a server address would flip the view at the first keystroke.
  let summaryCloud = $state(cloudSummary(draft));
  const termDuplicate = $derived(term.trim() !== '' && addTerm(draft.vocabulary, term) === null);
  // An excluded output stays listed while unplugged, so the choice survives.
  const outputChoices = $derived([
    ...outputs,
    ...draft.meeting.excluded_outputs.filter((id) => !outputs.some((o) => o.id === id)).map((id) => ({ id, name: id })),
  ]);
  const llmNotLocal = $derived(draft.llm.enabled && draft.processing_mode === 'local' && !isLoopbackUrl(draft.llm.base_url));

  async function loadModels() {
    models = (await attempt(api.listModels)) ?? models;
  }

  onMount(() => {
    loadModels();
    attempt(api.listOutputs).then((v) => (outputs = v ?? []));
    attempt(api.hasWebhookSecret).then((v) => (hasSecret = v ?? false));
    attempt(api.hasLlmApiKey).then((v) => (hasKeys.llm = v ?? false));
    attempt(api.hasMistralApiKey).then((v) => (hasKeys.mistral = v ?? false));
    const off = events.modelProgress(({ id, progress: p }) => {
      progress[id] = p;
      if (p >= 1) {
        delete progress[id];
        loadModels();
      }
    });
    return () => off.then((f) => f());
  });

  function toggleOutput(id: string) {
    const ex = draft.meeting.excluded_outputs;
    draft.meeting.excluded_outputs = ex.includes(id) ? ex.filter((x) => x !== id) : [...ex, id];
  }

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

  function addVocabulary(e: SubmitEvent) {
    e.preventDefault();
    const next = addTerm(draft.vocabulary, term);
    if (!next) return;
    draft.vocabulary = next;
    term = '';
  }

  function useLocal() {
    summaryCloud = false;
    draft.processing_mode = modeFor(draft.meeting.model, false);
    draft.llm.base_url = OLLAMA_URL;
  }

  function useCloud() {
    summaryCloud = true;
    draft.processing_mode = modeFor(draft.meeting.model, true);
    draft.llm.base_url = OPENROUTER_URL;
    // An Ollama tag (no vendor prefix) would be refused by OpenRouter.
    if (!draft.llm.model.includes('/')) draft.llm.model = CLOUD_MODELS[0];
  }

  function pickCloud(e: MouseEvent) {
    if (summaryCloud) return;
    if (draft.cloud_disclosure_acknowledged) return useCloud();
    // Keep the radio on "local" until the user has read what leaves the machine.
    e.preventDefault();
    disclosureOpen = true;
  }

  function acknowledgeCloud() {
    draft.cloud_disclosure_acknowledged = true;
    disclosureOpen = false;
    useCloud();
  }

  async function storeKey(kind: 'llm' | 'mistral', value: string): Promise<boolean> {
    const set = kind === 'llm' ? api.setLlmApiKey : api.setMistralApiKey;
    value = value.trim();
    if ((await attempt(() => set(value))) === undefined) return false;
    hasKeys[kind] = value !== '';
    keys[kind] = '';
    notify(t(value ? 'settings.key_saved' : 'settings.key_cleared'));
    return true;
  }

  function pickMeetingModel(id: string) {
    const previous = draft.meeting.model;
    draft.meeting.model = id;
    if (id === MISTRAL_MODEL) {
      // Audio leaves the machine: nothing changes until the disclosure is confirmed (and a key given if none is saved).
      mistralFallback = previous;
      mistralOpen = true;
      return;
    }
    draft.processing_mode = modeFor(id, summaryCloud);
  }

  function cancelMistral() {
    mistralOpen = false;
    draft.meeting.model = mistralFallback;
  }

  function activateMistral() {
    hasKeys.mistral = true;
    mistralOpen = false;
    draft.processing_mode = modeFor(MISTRAL_MODEL, summaryCloud);
  }

  // Persists only the flag from the saved settings: pending draft edits are neither committed nor lost to a refusal.
  async function replayOnboarding() {
    const next = { ...$state.snapshot(app.settings!), onboarding_done: false };
    if ((await attempt(() => api.saveSettings(next))) === undefined) return;
    applySettings(next);
    go({ name: 'onboarding' });
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

    <div class="groups">
      <section class="group" aria-labelledby="s-dictation">
        <h2 id="s-dictation">{t('settings.dictation')}</h2>
        <div class="box">
          <fieldset class="line stack">
            <legend class="secondary">{t('settings.mode')}</legend>
            {#each ['toggle', 'hold'] as const as mode (mode)}
              <label class="choice">
                <input type="radio" name="dictation-mode" value={mode} bind:group={draft.dictation.mode} />
                <span><span class="strong">{t(`settings.${mode}`)}</span> <span class="secondary">{t(`settings.${mode}_hint`)}</span></span>
              </label>
            {/each}
          </fieldset>
          <label class="line">
            <span class="lbl">{t('settings.model')}<span id="dictation-model-hint" class="secondary">{t('settings.dictation_model_hint')}</span></span>
            <select class="ctl" bind:value={draft.dictation.model} aria-describedby="dictation-model-hint">
              {#if parakeet.length}
                <optgroup label={t('settings.group_fast')}>
                  {#each parakeet as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
                </optgroup>
              {/if}
              <optgroup label="Whisper">
                {#each whisper as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
              </optgroup>
            </select>
          </label>
          <div class="line">
            <span class="lbl"><label for="show-overlay">{t('settings.show_overlay')}</label><span class="secondary" id="overlay-hint">{t('settings.show_overlay_hint')}</span></span>
            <input id="show-overlay" type="checkbox" bind:checked={draft.dictation.show_overlay} aria-describedby="overlay-hint" />
          </div>
          <label class="line">
            <span class="lbl">{t('settings.delay_before')}</span>
            <input class="num" type="number" min="0" max="2000" step="10" bind:value={draft.dictation.paste_delay_before_ms} />
          </label>
          <label class="line">
            <span class="lbl">{t('settings.delay_after')}</span>
            <input class="num" type="number" min="0" max="5000" step="50" bind:value={draft.dictation.paste_delay_after_ms} />
          </label>
          <label class="line">
            <span class="lbl">{t('settings.keep_history')}</span>
            <input type="checkbox" bind:checked={draft.dictation.keep_history} />
          </label>
        </div>
      </section>

      <section class="group" aria-labelledby="s-meeting">
        <h2 id="s-meeting">{t('settings.meeting')}</h2>
        <div class="box">
          <label class="line">
            <span class="lbl">{t('settings.model')}</span>
            <select class="ctl" value={draft.meeting.model} onchange={(e) => pickMeetingModel(e.currentTarget.value)}>
              {#each whisper as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
              <option value={MISTRAL_MODEL}>{t('settings.model_mistral')}</option>
            </select>
          </label>
          {#if draft.meeting.model === MISTRAL_MODEL && !mistralOpen}{@render keyForm('mistral', t('mistral.key'))}{/if}
          <label class="line">
            <span class="lbl">
              <span>{t('settings.threshold')} <output class="tabular">{new Intl.NumberFormat(app.lang, { minimumFractionDigits: 2 }).format(draft.meeting.confidence_threshold)}</output></span>
              <span id="threshold-hint" class="secondary">{t('settings.threshold_hint')}</span>
            </span>
            <input class="ctl" type="range" min="0.3" max="0.9" step="0.05" bind:value={draft.meeting.confidence_threshold} aria-describedby="threshold-hint" />
          </label>
          <fieldset class="line stack" aria-describedby="outputs-hint">
            <legend class="secondary">{t('settings.outputs')}</legend>
            {#each outputChoices as o (o.id)}
              <label class="check"><input type="checkbox" checked={!draft.meeting.excluded_outputs.includes(o.id)} onchange={() => toggleOutput(o.id)} /> {o.name}</label>
            {:else}
              <p class="secondary">{t('settings.outputs_none')}</p>
            {/each}
            <span id="outputs-hint" class="secondary">{t('settings.outputs_hint')}</span>
          </fieldset>
          <label class="line">
            <span class="lbl">{t('settings.retention')}</span>
            <select class="ctl" bind:value={draft.meeting.retention}>
              {#each ['on_review', 'seven_days', 'forever'] as r (r)}<option value={r}>{t(`settings.retention.${r}`)}</option>{/each}
            </select>
          </label>
        </div>
      </section>

      <section class="group" aria-labelledby="s-vocab">
        <h2 id="s-vocab">{t('settings.vocabulary')}</h2>
        <div class="box">
          <div class="line stack">
            <p class="secondary">{t('settings.vocabulary_hint')}</p>
            {#if draft.vocabulary.length}
              <ul class="terms">
                {#each draft.vocabulary as v, i (v)}
                  <li>
                    <span>{v}</span>
                    <button class="btn ghost icon remove" aria-label={t('settings.term_remove', { term: v })} onclick={() => draft.vocabulary.splice(i, 1)}>
                      <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M2 2 L10 10 M10 2 L2 10" /></svg>
                    </button>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="secondary">{t('settings.vocabulary_empty')}</p>
            {/if}
          </div>
          <form class="line secret" onsubmit={addVocabulary}>
            <label class="field grow"><span>{t('settings.term')}</span>
              <input type="text" bind:value={term} autocomplete="off" aria-describedby="term-state" />
            </label>
            <button class="btn" type="submit" disabled={!term.trim() || termDuplicate}>{t('settings.term_add')}</button>
          </form>
        </div>
        <p class="secondary note" id="term-state" role="status">{termDuplicate ? t('settings.term_duplicate') : ''}</p>
      </section>

      <section class="group" aria-labelledby="s-models">
        <h2 id="s-models">{t('settings.models')}</h2>
        <ul class="box">
          {#each models as m (m.id)}
            {@const p = progress[m.id]}
            <li class="line">
              <div class="lbl">
                <span class="strong">{m.name}</span>
                <span class="secondary">{t(`settings.kind.${m.kind}`)} · <span class="tabular">{size(m.size_mb)}</span></span>
                {#if p !== undefined}
                  <progress max="1" value={p} aria-label={t('settings.downloading', { pct: Math.round(p * 100) })}></progress>
                {/if}
              </div>
              {#if p !== undefined}
                <span class="secondary tabular" aria-live="polite">{p >= 0.99 ? t('settings.installing') : t('settings.downloading', { pct: Math.round(p * 100) })}</span>
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

      <section class="group" aria-labelledby="s-shortcuts">
        <h2 id="s-shortcuts">{t('settings.shortcuts')}</h2>
        <div class="box">
          {#each ['dictation', 'meeting_toggle', 'marker', 'review_queue'] as const as k (k)}
            <div class="line plain"><ShortcutInput label={t(`settings.sc.${k}`)} bind:value={draft.shortcuts[k]} /></div>
          {/each}
        </div>
        <p class="secondary note">{t('settings.sc.hint')}</p>
      </section>

      <section class="group" aria-labelledby="s-dest">
        <h2 id="s-dest">{t('settings.destinations')}</h2>
        <div class="box">
          <label class="line">
            <span class="lbl">{t('settings.folder_enabled')}</span>
            <input type="checkbox" bind:checked={draft.destinations.folder.enabled} />
          </label>
          <label class="line">
            <span class="lbl">{t('settings.folder_path')}</span>
            <input class="ctl" type="text" bind:value={draft.destinations.folder.path} disabled={!draft.destinations.folder.enabled} />
          </label>
        </div>
        <div class="box">
          <label class="line">
            <span class="lbl">{t('settings.webhook_enabled')}</span>
            <input type="checkbox" bind:checked={draft.destinations.webhook.enabled} />
          </label>
          <label class="line">
            <span class="lbl">{t('settings.webhook_url')}</span>
            <input class="ctl" type="url" bind:value={draft.destinations.webhook.url} placeholder="https://" disabled={!draft.destinations.webhook.enabled} />
          </label>
          <form class="line secret" onsubmit={(e) => { e.preventDefault(); if (secret) storeSecret(secret); }}>
            <label class="field grow"><span>{t('settings.secret')}</span>
              <input type="password" bind:value={secret} autocomplete="off" aria-describedby="secret-state" />
            </label>
            <button class="btn" type="submit" disabled={!secret}>{t('settings.secret_save')}</button>
          </form>
          <p class="line secondary" id="secret-state">
            {t(hasSecret ? 'settings.secret_set' : 'settings.secret_none')}
            {#if hasSecret}<button class="btn ghost small danger" onclick={() => storeSecret('')}>{t('settings.secret_clear')}</button>{/if}
          </p>
        </div>
      </section>

      <section class="group" aria-labelledby="s-llm">
        <h2 id="s-llm">{t('settings.llm')}</h2>
        <div class="box">
          <label class="line">
            <span class="lbl">{t('settings.llm_enabled')}</span>
            <input type="checkbox" bind:checked={draft.llm.enabled} />
          </label>
          <fieldset class="line stack" disabled={!draft.llm.enabled}>
            <legend class="secondary">{t('settings.llm_where')}</legend>
            <label class="choice">
              <input type="radio" name="llm-where" checked={!summaryCloud} onchange={useLocal} />
              <span><span class="strong">{t('settings.llm_local')}</span> <span class="secondary">{t('settings.llm_local_hint')}</span></span>
            </label>
            <label class="choice">
              <input type="radio" name="llm-where" checked={summaryCloud} onclick={pickCloud} />
              <span><span class="strong">{t('settings.llm_cloud')}</span> <span class="secondary">{t('settings.llm_cloud_hint')}</span></span>
            </label>
          </fieldset>
          {#if summaryCloud}
            <label class="line">
              <span class="lbl">{t('settings.llm_model')}<span id="cloud-model-hint" class="secondary">{t('settings.llm_cloud_model_hint')}</span></span>
              <input class="ctl" type="text" list="cloud-models" bind:value={draft.llm.model} disabled={!draft.llm.enabled} autocomplete="off" aria-describedby="cloud-model-hint" />
              <datalist id="cloud-models">{#each CLOUD_MODELS as m (m)}<option value={m}></option>{/each}</datalist>
            </label>
            {@render keyForm('llm', t('settings.api_key'))}
          {:else}
            <label class="line">
              <span class="lbl">{t('settings.base_url')}</span>
              <input class="ctl" type="url" bind:value={draft.llm.base_url} disabled={!draft.llm.enabled} />
            </label>
            <label class="line">
              <span class="lbl">{t('settings.llm_model')}</span>
              <input class="ctl" type="text" bind:value={draft.llm.model} disabled={!draft.llm.enabled} />
            </label>
            <div class="line stack">
              <p class="secondary">{t('settings.llm_local_note')}</p>
              {#if llmNotLocal}<p class="warn" role="status">{t('settings.llm_not_local')}</p>{/if}
            </div>
          {/if}
        </div>
      </section>

      <section class="group" aria-labelledby="s-lang">
        <h2 id="s-lang">{t('settings.language')}</h2>
        <div class="box">
          <label class="line">
            <span class="lbl">{t('settings.ui_language')}</span>
            <select class="ctl" bind:value={draft.ui_language}>
              <option value="fr">Français</option>
              <option value="en">English</option>
            </select>
          </label>
          <label class="line">
            <span class="lbl">
              {t('settings.transcription_language')}
              {#if draft.meeting.model === MISTRAL_MODEL}<span id="lang-mistral-hint" class="secondary">{t('settings.transcription_language_mistral')}</span>{/if}
            </span>
            <select class="ctl" bind:value={draft.transcription_language} aria-describedby={draft.meeting.model === MISTRAL_MODEL ? 'lang-mistral-hint' : undefined}>
              {#each ['auto', 'fr', 'en'] as l (l)}<option value={l}>{t(`settings.lang.${l}`)}</option>{/each}
            </select>
          </label>
        </div>
      </section>

      <section class="group" aria-labelledby="s-misc">
        <h2 id="s-misc">{t('settings.misc')}</h2>
        <div class="box">
          <label class="line">
            <span class="lbl">{t('settings.notification_sound')}</span>
            <input type="checkbox" bind:checked={draft.notification_sound} />
          </label>
          <div class="line">
            <span class="lbl"><label for="show-mascot">{t('settings.mascot')}</label><span class="secondary" id="mascot-hint">{t('settings.mascot_hint')}</span></span>
            <input id="show-mascot" type="checkbox" checked={app.mascot} onchange={(e) => setMascot(e.currentTarget.checked)} aria-describedby="mascot-hint" />
          </div>
        </div>
      </section>

      <section class="group" aria-labelledby="s-welcome">
        <h2 id="s-welcome">{t('settings.welcome')}</h2>
        <div class="box">
          <div class="line">
            <p class="lbl secondary" id="welcome-hint">{t('settings.welcome_hint')}</p>
            <button class="btn" aria-describedby="welcome-hint" disabled={saving} onclick={replayOnboarding}>{t('settings.welcome_again')}</button>
          </div>
        </div>
      </section>
    </div>
  </div>
</section>

{#snippet keyForm(kind: 'llm' | 'mistral', label: string)}
  <form class="line secret" onsubmit={(e) => { e.preventDefault(); if (keys[kind].trim()) storeKey(kind, keys[kind]); }}>
    <label class="field grow"><span>{label}</span>
      <input type="password" bind:value={keys[kind]} autocomplete="off" aria-describedby="{kind}-key-state" />
    </label>
    <button class="btn" type="submit" disabled={!keys[kind]}>{t('settings.key_save')}</button>
  </form>
  <p class="line secondary" id="{kind}-key-state">
    {t(hasKeys[kind] ? 'settings.key_set' : 'settings.key_none')}
    {#if hasKeys[kind]}<button class="btn ghost small danger" onclick={() => storeKey(kind, '')}>{t('settings.key_clear')}</button>{/if}
  </p>
{/snippet}

<MistralSheet open={mistralOpen} hasKey={hasKeys.mistral} oncancel={cancelMistral} onconfirm={activateMistral} />

<Sheet open={disclosureOpen} title={t('cloud.title')} onclose={() => (disclosureOpen = false)}>
  <p>{t('cloud.intro')}</p>
  <ul class="bullets">
    <li>{t('cloud.sent_transcript')}</li>
    <li>{t('cloud.sent_notes')}</li>
    <li>{t('cloud.sent_vocabulary')}</li>
  </ul>
  <p>{t(draft.meeting.model === MISTRAL_MODEL ? 'cloud.never_mistral' : 'cloud.never')}</p>
  <p class="secondary">{t('cloud.terms')}</p>
  {#snippet actions()}
    <button class="btn" onclick={() => (disclosureOpen = false)}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={acknowledgeCloud}>{t('cloud.confirm')}</button>
  {/snippet}
</Sheet>

<style>
  /* macOS-like settings: a title per group, then one grouped box of label/control lines. */
  .groups { display: grid; gap: 32px; max-width: 680px; }
  .group { display: grid; gap: 8px; }
  .group > h2 { padding-left: 4px; }
  .box { background: var(--fill); border-radius: var(--r-card); }
  .box > * + * { border-top: 1px solid var(--border); }
  .line {
    display: flex; align-items: center; justify-content: space-between; gap: 16px;
    min-height: 44px; padding: 8px 16px; margin: 0;
  }
  .lbl { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .ctl { width: 280px; flex: none; }
  /* Selects grow to their longest option rather than clip it. */
  select.ctl { width: auto; min-width: 280px; max-width: 100%; }
  .num { width: 112px; flex: none; text-align: right; }
  .line > input[type="checkbox"] { width: 16px; height: 16px; margin: 0; flex: none; }
  .stack { display: grid; justify-content: stretch; gap: 8px; padding-block: 12px; }
  .plain { display: block; }
  fieldset { border: 0; min-width: 0; }
  legend { float: left; width: 100%; padding: 0; }
  .choice { display: flex; gap: 8px; align-items: baseline; }
  .check { display: flex; gap: 8px; align-items: center; }
  .strong { font-weight: 550; }
  .grow { flex: 1; min-width: 0; }
  output { color: var(--text); font-weight: 550; margin-left: 4px; }
  progress { width: 100%; height: 4px; margin-top: 4px; accent-color: var(--accent); }
  .installed { font-size: 12.5px; color: var(--ok); }
  .secret { align-items: end; padding-block: 12px; }
  .warn { color: var(--warn); font-size: 13px; }
  .note { padding: 0 4px; }
  /* The empty live region stays in the tree (so it announces) but takes no room. */
  .note:empty { margin-top: -8px; }
  .terms { display: flex; flex-wrap: wrap; gap: 4px; }
  .terms li {
    display: inline-flex; align-items: center; gap: 2px;
    padding: 2px 2px 2px 12px; border-radius: var(--r-control);
    background: var(--surface); border: 1px solid var(--border);
  }
  .remove { width: 24px; min-height: 24px; }
  .remove path { stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
  .bullets { list-style: disc; padding-left: 20px; display: grid; gap: 4px; }
  .error {
    max-width: 680px; margin-bottom: 16px; padding: 12px 16px; border-radius: var(--r-card);
    color: var(--danger); border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    background: color-mix(in srgb, var(--danger) 8%, var(--surface));
  }
  @media (max-width: 860px) { .ctl { width: 220px; } select.ctl { min-width: 220px; } }
</style>
