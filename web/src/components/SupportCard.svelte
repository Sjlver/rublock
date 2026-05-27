<script lang="ts">
  import { onMount } from 'svelte';
  import { t, tf } from '../i18n/index.svelte';
  import { trackSupportImpression, trackSupportClick, type SupportPrompt } from '../state/support';

  interface Props {
    prompt: SupportPrompt;
    onDismiss: () => void;
  }

  let { prompt, onDismiss }: Props = $props();

  // The card is only mounted when PlayTab decides to show it (a prime-numbered
  // solve), so mounting == one impression. `{#key prompt}` in the parent gives
  // each new prompt a fresh mount, so a re-solve fires a fresh impression.
  onMount(() => trackSupportImpression(prompt));

  function handleClick(): void {
    // Fire-and-forget; don't preventDefault, so the link still navigates.
    trackSupportClick(prompt);
  }
</script>

<aside
  class="support-card card"
  data-support-card
  data-support-platform={prompt.platform.id}
  data-support-copy={prompt.copyIndex}
>
  <button
    type="button"
    class="support-dismiss"
    onclick={onDismiss}
    aria-label={t('support_dismiss_aria')}
  >
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
    >
      <path d="M6 6l12 12M18 6L6 18" />
    </svg>
  </button>

  <p class="support-copy">{t(prompt.copyKey)}</p>

  <a
    class="btn-primary support-link"
    href={prompt.platform.url}
    target="_blank"
    rel="noopener noreferrer"
    onclick={handleClick}
  >
    <!-- Heart icon -->
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path
        d="M12 21s-6.7-4.35-9.33-8.07C1.3 10.7 1.6 7.6 3.9 6.05c1.86-1.26 4.3-.77 5.6.94L12 10l2.5-3.01c1.3-1.71 3.74-2.2 5.6-.94 2.3 1.55 2.6 4.65.93 6.88C18.7 16.65 12 21 12 21z"
      />
    </svg>
    {tf('support_button', { platform: prompt.platform.label })}
  </a>
</aside>

<style>
  .support-card {
    position: relative;
    margin: 1.25rem auto 0;
    max-width: 360px;
    text-align: center;
    /* Reserve space so the card appearing doesn't shift surrounding layout. */
    min-height: 96px;
  }
  .support-copy {
    font-size: 13.5px;
    line-height: 1.45;
    color: var(--ink-2);
    /* Leave room on the right so long copy never collides with the dismiss X. */
    margin: 2px 20px 12px;
  }
  .support-link {
    text-decoration: none;
  }
  .support-dismiss {
    position: absolute;
    top: 8px;
    right: 8px;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    font-family: inherit;
  }
  .support-dismiss:hover {
    color: var(--ink);
  }
</style>
