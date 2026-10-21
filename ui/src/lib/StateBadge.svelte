<script lang="ts">
  import type { MeetingState } from './api';
  import { t } from './i18n';

  let { state }: { state: MeetingState } = $props();

  const tone: Record<MeetingState, string> = {
    recording: 'danger', captured: 'neutral', queued: 'neutral', processing: 'neutral',
    needs_review: 'warn', reviewed: 'ok', sending: 'accent', sent: 'ok', failed: 'danger',
  };
</script>

<span class="badge {tone[state]}">{t(`state.${state}`)}</span>

<style>
  .badge {
    display: inline-block; padding: 1px 8px; border-radius: 999px;
    font-size: 12px; font-weight: 500; line-height: 18px; white-space: nowrap;
    color: var(--c); background: color-mix(in srgb, var(--c) 13%, transparent);
  }
  .neutral { --c: var(--text-2); }
  .warn { --c: var(--warn); }
  .ok { --c: var(--ok); }
  .danger { --c: var(--danger); }
  .accent { --c: var(--accent); }
</style>
