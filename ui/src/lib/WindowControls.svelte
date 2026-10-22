<script lang="ts">
  // Windows and Linux draw no system title bar (tauri.conf.json): these replace its buttons.
  // macOS keeps its own traffic lights over the content (tauri.macos.conf.json).
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { t } from './i18n';

  // Close hides to the tray (CloseRequested in src-tauri/src/lib.rs).
  const actions = [
    { key: 'window.minimize', run: () => getCurrentWindow().minimize(), d: 'M3.5 8h9' },
    { key: 'window.maximize', run: () => getCurrentWindow().toggleMaximize(), d: 'M4 4h8v8H4Z' },
    { key: 'window.close', run: () => getCurrentWindow().close(), d: 'M4 4l8 8M12 4l-8 8' },
  ];
</script>

<div class="controls">
  {#each actions as a (a.key)}
    <button class:close={a.key === 'window.close'} aria-label={t(a.key)} title={t(a.key)} onclick={() => a.run().catch(() => {})}>
      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d={a.d} /></svg>
    </button>
  {/each}
</div>

<style>
  .controls { position: fixed; top: 0; right: 0; z-index: 50; display: flex; }
  button {
    display: grid; place-items: center; width: 44px; height: 28px;
    border: 0; background: transparent; color: var(--text-2); cursor: default;
    transition: background-color 150ms var(--ease), color 150ms var(--ease);
  }
  button:hover { background: var(--fill-strong); color: var(--text); }
  button.close:hover { background: var(--danger); color: var(--on-accent); }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  path { fill: none; stroke: currentColor; stroke-width: 1.25; stroke-linecap: round; stroke-linejoin: round; }
</style>
