<script lang="ts">
  import { onMount } from 'svelte';
  import { t, tf } from '../i18n/index.svelte';
  import { trackSupportImpression, trackSupportClick, type SupportPrompt } from '../state/support';

  interface Props {
    prompt: SupportPrompt;
    onShare: () => Promise<void>;
    onDismiss: () => void;
  }

  let { prompt, onShare, onDismiss }: Props = $props();

  let action = $state<HTMLElement>();

  // The card is only mounted when PlayTab decides to show it (a prime-numbered
  // solve), so mounting == one impression. `{#key prompt}` in the parent gives
  // each new prompt a fresh mount, so a re-solve fires a fresh impression.
  onMount(() => {
    trackSupportImpression(prompt);
    action?.focus();
  });

  function handleDonateClick(): void {
    // Fire-and-forget; don't preventDefault, so the link still navigates.
    trackSupportClick(prompt);
    onDismiss();
  }

  async function handleShareClick(): Promise<void> {
    trackSupportClick(prompt);
    await onShare();
    // Close afterwards so the "link copied" toast in the header is visible.
    onDismiss();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      onDismiss();
    }
  }

  function handleBackdropClick(event: MouseEvent): void {
    if (event.target === event.currentTarget) onDismiss();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="support-backdrop" onclick={handleBackdropClick} role="presentation">
  <div
    class="support-card card"
    role="dialog"
    aria-modal="true"
    aria-labelledby="support-heading"
    data-support-card
    data-support-channel={prompt.channel.id}
    data-support-copy={prompt.copyIndex}
  >
    <button
      type="button"
      class="support-dismiss"
      onclick={onDismiss}
      aria-label={t('support_dismiss_aria')}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
      >
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>

    <h2 id="support-heading" class="support-heading">{t('support_heading')}</h2>
    <p class="support-copy">{t(prompt.copyKey)}</p>

    {#if prompt.channel.kind === 'donate'}
      <a
        bind:this={action}
        class="btn-primary support-action"
        href={prompt.channel.url}
        target="_blank"
        rel="noopener noreferrer"
        onclick={handleDonateClick}
      >
        <!-- Heart icon -->
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path
            d="M12 21s-6.7-4.35-9.33-8.07C1.3 10.7 1.6 7.6 3.9 6.05c1.86-1.26 4.3-.77 5.6.94L12 10l2.5-3.01c1.3-1.71 3.74-2.2 5.6-.94 2.3 1.55 2.6 4.65.93 6.88C18.7 16.65 12 21 12 21z"
          />
        </svg>
        {tf('support_button', { platform: prompt.channel.label })}
      </a>
    {:else}
      <button
        bind:this={action}
        type="button"
        class="btn-primary support-action"
        onclick={handleShareClick}
      >
        <!-- Share icon -->
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <circle cx="18" cy="5" r="2.5" />
          <circle cx="6" cy="12" r="2.5" />
          <circle cx="18" cy="19" r="2.5" />
          <path d="M8.2 10.8l7.6-4.4M8.2 13.2l7.6 4.4" />
        </svg>
        {t('support_share_button')}
      </button>
    {/if}

    <button type="button" class="support-later" onclick={onDismiss}>
      {t('support_not_now')}
    </button>
  </div>
</div>

<style>
  /* Above the app (incl. the z-index 20 menus), below the emoji rain (9999). */
  .support-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: color-mix(in oklch, var(--bg-page) 80%, transparent);
    backdrop-filter: blur(6px);
    animation: support-fade 450ms ease-out both;
  }
  .support-card {
    position: relative;
    width: 100%;
    max-width: 380px;
    padding: 28px 24px 16px;
    text-align: center;
    animation: support-rise 450ms ease-out both;
  }
  .support-heading {
    font-size: 22px;
    font-weight: 700;
    color: var(--ink);
    margin-bottom: 8px;
  }
  .support-copy {
    font-size: 15px;
    line-height: 1.5;
    color: var(--ink-2);
    margin: 0 8px 20px;
  }
  .support-action {
    text-decoration: none;
  }
  .support-later {
    margin-top: 8px;
    height: 36px;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--muted);
    font-family: inherit;
    font-size: 13.5px;
    cursor: pointer;
  }
  .support-later:hover {
    color: var(--ink);
  }
  .support-dismiss {
    position: absolute;
    top: 10px;
    right: 10px;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    font-family: inherit;
  }
  .support-dismiss:hover {
    color: var(--ink);
  }

  @keyframes support-fade {
    from {
      opacity: 0;
    }
  }
  @keyframes support-rise {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.98);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .support-backdrop,
    .support-card {
      animation-duration: 1ms;
    }
  }
</style>
