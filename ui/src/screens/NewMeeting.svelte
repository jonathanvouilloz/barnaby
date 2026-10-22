<script lang="ts">
  import { api, type Participant } from '../lib/api';
  import { app, applySettings, attempt, go, notify } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import Sheet from '../lib/Sheet.svelte';

  let title = $state('');
  let participants = $state<Participant[]>([]);
  let name = $state('');
  let email = $state('');
  let starting = $state(false);
  let nameInput: HTMLInputElement;

  const consentNeeded = $derived(!app.settings!.consent_acknowledged);
  const defaultTitle = t('new.name_default', {
    date: new Intl.DateTimeFormat(app.lang, { day: 'numeric', month: 'long' }).format(new Date()),
  });

  function add(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    participants.push({ id: crypto.randomUUID(), name: name.trim(), email: email.trim() || null, confirmed: true });
    name = '';
    email = '';
    nameInput.focus();
  }

  async function acknowledge() {
    const next = { ...$state.snapshot(app.settings!), consent_acknowledged: true };
    if ((await attempt(() => api.saveSettings(next))) !== undefined) applySettings(next);
  }

  async function copyTemplate() {
    await attempt(() => navigator.clipboard.writeText(t('consent.template')));
    notify(t('common.copied'));
  }

  async function start() {
    starting = true;
    const id = await attempt(() => api.startMeeting(title.trim() || defaultTitle, $state.snapshot(participants)));
    starting = false;
    if (id) {
      app.recordingId = id;
      go({ name: 'recording' });
    }
  }
</script>

<section class="screen">
  <header class="screen-head" data-tauri-drag-region>
    <button class="btn ghost" onclick={() => go({ name: 'home' })}>{t('common.back')}</button>
    <h1>{t('new.title')}</h1>
    <button class="btn primary" disabled={starting || consentNeeded} onclick={start}><span class="rec" aria-hidden="true"></span>{t('new.start')}</button>
  </header>

  <div class="screen-body">
    <div class="form">
      <label class="field">
        <span>{t('new.name')}</span>
        <input type="text" bind:value={title} placeholder={defaultTitle} />
      </label>

      <section class="people" aria-labelledby="people-title">
        <div>
          <h2 id="people-title">{t('new.participants')}</h2>
          <p class="secondary">{t('new.participants_hint')}</p>
        </div>

        {#if participants.length === 0}
          <p class="secondary none">{t('new.none')}</p>
        {:else}
          <ul>
            {#each participants as p, i (p.id)}
              <li>
                <span class="name">{p.name}</span>
                {#if p.email}<span class="secondary">{p.email}</span>{/if}
                <button class="btn ghost small" aria-label={t('new.remove', { name: p.name })} onclick={() => participants.splice(i, 1)}>
                  {t('new.remove_short')}
                </button>
              </li>
            {/each}
          </ul>
        {/if}

        <form class="add" onsubmit={add}>
          <label class="field"><span>{t('new.p_name')}</span><input type="text" bind:value={name} bind:this={nameInput} autocomplete="off" /></label>
          <label class="field"><span>{t('new.p_email')}</span><input type="email" bind:value={email} autocomplete="off" /></label>
          <button class="btn" type="submit" disabled={!name.trim()}>{t('new.add')}</button>
        </form>
      </section>
    </div>
  </div>
</section>

<Sheet open={consentNeeded} title={t('consent.title')} onclose={() => go({ name: 'home' })}>
  <p>{t('consent.body')}</p>
  <blockquote class="template">{t('consent.template')}</blockquote>
  <div><button class="btn small" onclick={copyTemplate}>{t('consent.copy')}</button></div>
  {#snippet actions()}
    <button class="btn" onclick={() => go({ name: 'home' })}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={acknowledge}>{t('consent.ack')}</button>
  {/snippet}
</Sheet>

<style>
  .form { display: grid; gap: 40px; max-width: 560px; padding-top: 8px; }
  .people { display: grid; gap: 16px; }
  ul { display: grid; gap: 2px; }
  li {
    display: flex; align-items: center; gap: 12px; min-height: 40px; padding: 4px 8px;
    border-radius: var(--r-control); transition: background-color 150ms var(--ease);
  }
  li:hover { background: var(--fill); }
  .name { font-weight: 500; }
  li .btn { margin-left: auto; }
  .none { padding: 4px 0; }
  .add { display: grid; grid-template-columns: 1fr 1fr auto; gap: 12px; align-items: end; }
  .rec { width: 8px; height: 8px; border-radius: 50%; background: var(--rec); }
  .template {
    margin: 0; padding: 12px 16px; border-radius: var(--r-card);
    background: var(--fill); font-size: 13.5px; user-select: text;
  }
  @media (max-width: 720px) { .add { grid-template-columns: 1fr; } }
</style>
