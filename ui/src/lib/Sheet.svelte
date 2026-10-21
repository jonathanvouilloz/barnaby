<script lang="ts">
  // Modal sheet on the native <dialog>: focus trap, Escape and top layer come for free.
  import type { Snippet } from 'svelte';

  let { open, title, onclose, children, actions }: {
    open: boolean; title: string; onclose: () => void; children: Snippet; actions: Snippet;
  } = $props();

  let dialog: HTMLDialogElement;
  const id = $props.id();

  $effect(() => {
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} aria-labelledby="{id}-title" onclose={() => open && onclose()}>
  <h2 id="{id}-title">{title}</h2>
  <div class="content">{@render children()}</div>
  <div class="actions">{@render actions()}</div>
</dialog>

<style>
  dialog {
    width: min(480px, calc(100vw - 48px));
    padding: 20px 22px 18px;
    border: 1px solid var(--border);
    border-radius: var(--r-sheet);
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(24px) saturate(1.6);
    color: var(--text);
    box-shadow: 0 18px 50px rgb(0 0 0 / .25);
  }
  dialog[open] { animation: rise 200ms var(--ease); }
  dialog::backdrop { background: rgb(0 0 0 / .28); }
  .content { margin: 10px 0 18px; display: grid; gap: 12px; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  @keyframes rise { from { opacity: 0; transform: translateY(8px); } }
</style>
