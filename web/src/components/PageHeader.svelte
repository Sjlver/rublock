<script lang="ts">
  interface Props {
    title: string;
    status?: string;
    statusTone?: 'default' | 'error' | 'success';
    onShare?: () => void;
  }

  let { title, status = '', statusTone = 'default', onShare }: Props = $props();
</script>

<div class="page-header">
  <div class="page-header-row">
    <h1 class="page-title">{title}</h1>
    {#if onShare}
      <button type="button" class="btn-share" onclick={onShare} aria-label="Share">
        <!-- Share icon -->
        <svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.7"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <circle cx="6" cy="12" r="2.4" />
          <circle cx="18" cy="6.5" r="2.4" />
          <circle cx="18" cy="17.5" r="2.4" />
          <path d="M8 11l8-3.5M8 13l8 3.5" />
        </svg>
        Share
      </button>
    {/if}
  </div>
  {#if status}
    <div
      class="page-status"
      class:error={statusTone === 'error'}
      class:success={statusTone === 'success'}
      role="status"
      aria-live="polite"
    >
      {status}
    </div>
  {/if}
</div>
