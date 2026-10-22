<script lang="ts">
  // First run, full window: one step at a time, each one skippable.
  import { fly } from 'svelte/transition';
  import { api } from '../lib/api';
  import { app, applySettings, attempt, go } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import Owl from '../lib/Owl.svelte';
  import { STEPS } from './steps';
  import Mic from './Mic.svelte';
  import Models from './Models.svelte';
  import DictationTest from './DictationTest.svelte';
  import Tour from './Tour.svelte';

  let i = $state(0);
  let dir = $state(1);
  // Steps that prove something (mic heard, models in, text dictated) lower it until done; Passer always works.
  let ready = $state(true);
  let finishing = $state(false);
  let downloading = $state(false);
  let heading = $state<HTMLElement>();

  const step = $derived(STEPS[i]);
  const last = $derived(i === STEPS.length - 1);
  const shift = matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 48;

  function move(d: 1 | -1) {
    dir = d;
    ready = true;
    i += d;
  }

  async function finish() {
    finishing = true;
    const next = { ...$state.snapshot(app.settings!), onboarding_done: true };
    if ((await attempt(() => api.saveSettings(next))) !== undefined) {
      applySettings(next);
      go({ name: 'home' });
    }
    finishing = false;
  }

  // Keyboard and screen-reader users land on the new step's title; the dictation test keeps its textarea focused.
  $effect(() => {
    if (step !== 'dictation') queueMicrotask(() => heading?.focus());
  });
</script>

<section class="onboarding" aria-label={t('onboarding.label')}>
  <header data-tauri-drag-region>
    <ol class="dots" aria-hidden="true">
      {#each STEPS as s, n (s)}<li class:past={n < i} class:current={n === i}></li>{/each}
    </ol>
    <p class="sr-only" aria-live="polite">{t('onboarding.progress', { n: i + 1, total: STEPS.length })}</p>
  </header>

  <div class="stage">
    {#key step}
      <div
        class="step"
        class:wide={step === 'tour'}
        in:fly={{ x: dir * shift, duration: shift && 240, delay: shift && 60 }}
        out:fly={{ x: -dir * shift, duration: shift && 180 }}
      >
        {#if step === 'welcome'}
          {#if app.mascot}<Owl state="idle" size={128} />{/if}
          <h1 tabindex="-1" bind:this={heading}>{t('onboarding.welcome.title')}</h1>
          <p class="lead">{t('onboarding.welcome.text')}</p>
          <p class="promise">{t('onboarding.welcome.promise')}</p>
          <p class="secondary">{t('onboarding.welcome.time')}</p>
        {:else if step === 'mic'}
          <h1 tabindex="-1" bind:this={heading}>{t('onboarding.mic.title')}</h1>
          <Mic bind:ready />
        {:else if step === 'models'}
          <h1 tabindex="-1" bind:this={heading}>{t('onboarding.models.title')}</h1>
          <Models bind:ready bind:busy={downloading} />
        {:else if step === 'dictation'}
          <h1 tabindex="-1" bind:this={heading}>{t('onboarding.dictation.title')}</h1>
          <DictationTest bind:ready />
        {:else if step === 'tour'}
          <h1 tabindex="-1" bind:this={heading}>{t('onboarding.tour.title')}</h1>
          <Tour />
        {:else}
          {#if app.mascot}<Owl state="success" size={128} />{/if}
          <h1 tabindex="-1" bind:this={heading}>{t('onboarding.done.title')}</h1>
          <p class="lead">{t('onboarding.done.text')}</p>
        {/if}
      </div>
    {/key}
  </div>

  <footer class:wide={step === 'tour'}>
    {#if i > 0}<button class="btn ghost" disabled={downloading} onclick={() => move(-1)}>{t('common.back')}</button>{/if}
    <span class="grow">{#if !ready}<span id="continue-hint" class="secondary">{t(`onboarding.${step}.unlock`)}</span>{/if}</span>
    {#if last}
      <button class="btn primary" disabled={finishing} onclick={finish}>{t('onboarding.done.go')}</button>
    {:else}
      <button class="btn ghost" disabled={downloading} onclick={() => move(1)}>{t('onboarding.skip')}</button>
      <button id="onboarding-continue" class="btn primary" disabled={!ready} aria-describedby={ready ? undefined : 'continue-hint'} onclick={() => move(1)}>{t('onboarding.continue')}</button>
    {/if}
  </footer>
</section>

<style>
  .onboarding {
    display: grid; grid-template-rows: auto 1fr auto;
    height: 100%; background: var(--bg);
  }
  header { display: flex; justify-content: center; padding: 28px 40px 0; min-height: 56px; }
  .dots { display: flex; gap: 8px; align-items: center; }
  .dots li {
    width: 6px; height: 6px; border-radius: 50%; background: var(--fill-strong);
    transition: width 240ms var(--ease), background-color 240ms var(--ease);
  }
  .dots li.past { background: var(--text-2); }
  .dots li.current { width: 20px; border-radius: 3px; background: var(--accent); }

  /* Both steps share one cell during the slide, so nothing jumps. */
  .stage { display: grid; min-height: 0; overflow: hidden auto; padding: 24px 40px; }
  .step {
    grid-area: 1 / 1; align-self: center; justify-self: center;
    width: min(520px, 100%); display: grid; gap: 16px; justify-items: start;
  }
  .step.wide { width: min(840px, 100%); }
  h1:focus { outline: none; }
  .lead { font-size: 16px; }
  .promise { font-weight: 550; }

  footer {
    display: flex; align-items: center; gap: 8px;
    width: min(600px, 100%); justify-self: center; padding: 16px 40px 32px;
  }
  footer.wide { width: min(920px, 100%); }
  .grow { flex: 1; text-align: right; }
</style>
