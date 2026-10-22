<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api, events, type ModelInfo, type OutputDevice, type Settings } from '../lib/api';
  import { app, applySettings, attempt, notify, setMascot } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { formatSize } from '../lib/format';
  import { addTerm, isLoopbackUrl, MISTRAL_MODEL, modeFor, OLLAMA_URL, OPENROUTER_URL } from '../lib/settings';
  import ShortcutInput from '../lib/ShortcutInput.svelte';
  import Sheet from '../lib/Sheet.svelte';

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
  let summaryCloud = $state(draft.processing_mode === 'hybrid' || (draft.processing_mode === 'api' && !isLoopbackUrl(draft.llm.base_url)));
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
    if ((await attempt(() => set(value))) === undefined) return false;
    hasKeys[kind] = value !== '';
    keys[kind] = '';
    notify(t(value ? 'settings.key_saved' : 'settings.key_cleared'));
    return true;
  }

  function pickMeetingModel(id: string) {
    const previous = draft.meeting.model;
    draft.meeting.model = id;
    if (id === MISTRAL_MODEL && !hasKeys.mistral) {
      // Audio leaves the machine: nothing changes until a key is given.
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

  async function activateMistral() {
    if (!(await storeKey('mistral', keys.mistral))) return;
    mistralOpen = false;
    draft.processing_mode = modeFor(MISTRAL_MODEL, summaryCloud);
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
          {#each ['toggle', 'hold'] as const as mode (mode)}
            <label class="choice">
              <input type="radio" name="dictation-mode" value={mode} bind:group={draft.dictation.mode} />
              <span><span class="strong">{t(`settings.${mode}`)}</span> <span class="secondary">{t(`settings.${mode}_hint`)}</span></span>
            </label>
          {/each}
        </fieldset>
        <label class="field"><span>{t('settings.model')}</span>
          <select bind:value={draft.dictation.model} aria-describedby="dictation-model-hint">
            {#if parakeet.length}
              <optgroup label={t('settings.group_fast')}>
                {#each parakeet as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
              </optgroup>
            {/if}
            <optgroup label="Whisper">
              {#each whisper as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
            </optgroup>
          </select>
          <span id="dictation-model-hint" class="secondary">{t('settings.dictation_model_hint')}</span>
        </label>
        <label class="check"><input type="checkbox" bind:checked={draft.dictation.show_overlay} aria-describedby="overlay-hint" /> {t('settings.show_overlay')}</label>
        <p class="secondary sub" id="overlay-hint">{t('settings.show_overlay_hint')}</p>
        <div class="two">
          <label class="field"><span>{t('settings.delay_before')}</span><input type="number" min="0" max="2000" step="10" bind:value={draft.dictation.paste_delay_before_ms} /></label>
          <label class="field"><span>{t('settings.delay_after')}</span><input type="number" min="0" max="5000" step="50" bind:value={draft.dictation.paste_delay_after_ms} /></label>
        </div>
        <label class="check"><input type="checkbox" bind:checked={draft.dictation.keep_history} /> {t('settings.keep_history')}</label>
      </section>

      <section class="card" aria-labelledby="s-meeting">
        <h2 id="s-meeting">{t('settings.meeting')}</h2>
        <label class="field"><span>{t('settings.model')}</span>
          <select value={draft.meeting.model} onchange={(e) => pickMeetingModel(e.currentTarget.value)}>
            {#each whisper as m (m.id)}<option value={m.id}>{modelLabel(m)}</option>{/each}
            <option value={MISTRAL_MODEL}>{t('settings.model_mistral')}</option>
          </select>
        </label>
        {#if draft.meeting.model === MISTRAL_MODEL && !mistralOpen}{@render keyForm('mistral', t('mistral.key'))}{/if}
        <label class="field">
          <span>{t('settings.threshold')} <output class="tabular">{new Intl.NumberFormat(app.lang, { minimumFractionDigits: 2 }).format(draft.meeting.confidence_threshold)}</output></span>
          <input type="range" min="0.3" max="0.9" step="0.05" bind:value={draft.meeting.confidence_threshold} aria-describedby="threshold-hint" />
          <span id="threshold-hint" class="secondary">{t('settings.threshold_hint')}</span>
        </label>
        <fieldset aria-describedby="outputs-hint">
          <legend class="secondary">{t('settings.outputs')}</legend>
          {#each outputChoices as o (o.id)}
            <label class="check"><input type="checkbox" checked={!draft.meeting.excluded_outputs.includes(o.id)} onchange={() => toggleOutput(o.id)} /> {o.name}</label>
          {:else}
            <p class="secondary">{t('settings.outputs_none')}</p>
          {/each}
          <span id="outputs-hint" class="secondary">{t('settings.outputs_hint')}</span>
        </fieldset>
        <label class="field"><span>{t('settings.retention')}</span>
          <select bind:value={draft.meeting.retention}>
            {#each ['on_review', 'seven_days', 'forever'] as r (r)}<option value={r}>{t(`settings.retention.${r}`)}</option>{/each}
          </select>
        </label>
      </section>

      <section class="card" aria-labelledby="s-vocab">
        <h2 id="s-vocab">{t('settings.vocabulary')}</h2>
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
        <form class="secret" onsubmit={addVocabulary}>
          <label class="field grow"><span>{t('settings.term')}</span>
            <input type="text" bind:value={term} autocomplete="off" aria-describedby="term-state" />
          </label>
          <button class="btn" type="submit" disabled={!term.trim() || termDuplicate}>{t('settings.term_add')}</button>
        </form>
        <p class="secondary" id="term-state" role="status">{termDuplicate ? t('settings.term_duplicate') : ''}</p>
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
        <fieldset disabled={!draft.llm.enabled}>
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
          <label class="field"><span>{t('settings.llm_model')}</span>
            <input type="text" list="cloud-models" bind:value={draft.llm.model} disabled={!draft.llm.enabled} autocomplete="off" aria-describedby="cloud-model-hint" />
            <datalist id="cloud-models">{#each CLOUD_MODELS as m (m)}<option value={m}></option>{/each}</datalist>
            <span id="cloud-model-hint" class="secondary">{t('settings.llm_cloud_model_hint')}</span>
          </label>
          {@render keyForm('llm', t('settings.api_key'))}
        {:else}
          <div class="two">
            <label class="field"><span>{t('settings.base_url')}</span><input type="url" bind:value={draft.llm.base_url} disabled={!draft.llm.enabled} /></label>
            <label class="field"><span>{t('settings.llm_model')}</span><input type="text" bind:value={draft.llm.model} disabled={!draft.llm.enabled} /></label>
          </div>
          <p class="secondary">{t('settings.llm_local_note')}</p>
          {#if llmNotLocal}<p class="warn" role="status">{t('settings.llm_not_local')}</p>{/if}
        {/if}
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

{#snippet keyForm(kind: 'llm' | 'mistral', label: string)}
  <form class="secret" onsubmit={(e) => { e.preventDefault(); if (keys[kind]) storeKey(kind, keys[kind]); }}>
    <label class="field grow"><span>{label}</span>
      <input type="password" bind:value={keys[kind]} autocomplete="off" aria-describedby="{kind}-key-state" />
    </label>
    <button class="btn" type="submit" disabled={!keys[kind]}>{t('settings.key_save')}</button>
  </form>
  <p class="secondary row" id="{kind}-key-state">
    {t(hasKeys[kind] ? 'settings.key_set' : 'settings.key_none')}
    {#if hasKeys[kind]}<button class="btn ghost small danger" onclick={() => storeKey(kind, '')}>{t('settings.key_clear')}</button>{/if}
  </p>
{/snippet}

<Sheet open={mistralOpen} title={t('mistral.title')} onclose={cancelMistral}>
  <p>{t('mistral.intro')}</p>
  <ul class="bullets">
    <li>{t('mistral.sent_audio')}</li>
    <li>{t('mistral.sent_vocabulary')}</li>
  </ul>
  <p>{t('mistral.price')}</p>
  <p class="secondary">{t('mistral.terms')}</p>
  <label class="field"><span>{t('mistral.key')}</span>
    <input type="password" bind:value={keys.mistral} autocomplete="off" />
  </label>
  {#snippet actions()}
    <button class="btn" onclick={cancelMistral}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!keys.mistral} onclick={activateMistral}>{t('mistral.confirm')}</button>
  {/snippet}
</Sheet>

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
  .sub { margin-top: -8px; padding-left: 24px; }
  .terms { display: flex; flex-wrap: wrap; gap: 6px; }
  .terms li {
    display: inline-flex; align-items: center; gap: 2px;
    padding: 2px 2px 2px 10px; border-radius: var(--r-control);
    background: var(--fill); border: 1px solid var(--border);
  }
  .remove { width: 24px; min-height: 24px; }
  .remove path { stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
  /* Empty live region stays in the tree (so it announces) but takes no room. */
  #term-state:empty { margin-top: -12px; }
  .bullets { list-style: disc; padding-left: 20px; display: grid; gap: 4px; }
  .error {
    max-width: 680px; margin-bottom: 16px; padding: 10px 14px; border-radius: var(--r-card);
    color: var(--danger); border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    background: color-mix(in srgb, var(--danger) 8%, var(--surface));
  }
</style>
