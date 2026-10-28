<script lang="ts">
  // Where validated meetings are written. On by default, so a first meeting never goes "nowhere".
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { t } from '../lib/i18n';

  const folder = app.settings!.destinations.folder;

  onMount(async () => {
    if (!folder.path) {
      folder.path = await api.defaultFolder().catch(() => '');
      folder.enabled = !!folder.path;
    }
  });

  async function browse() {
    const picked = await api.pickFolder(folder.path).catch(() => null);
    if (picked) folder.path = picked;
  }
</script>

<p class="lead">{t('onboarding.destination.text')}</p>
<label class="check"><input type="checkbox" role="switch" bind:checked={folder.enabled} /> {t('settings.folder_enabled')}</label>
<div class="pick">
  <label class="sr-only" for="onboarding-folder">{t('settings.folder_path')}</label>
  <input id="onboarding-folder" class="ctl" type="text" bind:value={folder.path} disabled={!folder.enabled} />
  <button class="btn" type="button" onclick={browse} disabled={!folder.enabled}>{t('settings.folder_browse')}</button>
</div>
<p class="secondary">{t('onboarding.destination.later')}</p>

<style>
  .check { display: flex; align-items: center; gap: 8px; }
  .pick { display: flex; gap: 8px; width: min(100%, 560px); }
  .pick input { flex: 1; min-width: 0; width: auto; }
</style>
