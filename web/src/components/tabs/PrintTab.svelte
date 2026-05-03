<script lang="ts">
  import PageHeader from '../PageHeader.svelte';

  // Per DESIGN_NOTES.md: Print tab has its own size setting, independent from Play.
  interface Props {
    onPrint: (size: number, pages: number) => Promise<void>;
    busy: boolean;
  }

  let { onPrint, busy }: Props = $props();

  let size = $state(6);
  let pages = $state(4);

  async function handlePrint(): Promise<void> {
    const clamped = Math.max(1, Math.min(20, Number.isFinite(pages) ? pages : 4));
    await onPrint(size, clamped);
  }
</script>

<PageHeader title="Print" status="Generate a printable booklet" />

<div class="tab-content">
  <div class="card">
    <div class="field-row">
      <div>
        <div class="field-label">Size</div>
      </div>
      <div class="size-selector" style="background:var(--paper);">
        {#each [5, 6, 7, 8] as s (s)}
          <button
            type="button"
            class="size-btn"
            class:active={s === size}
            onclick={() => (size = s)}
          >
            {s}×{s}
          </button>
        {/each}
      </div>
    </div>

    <div class="divider"></div>

    <div class="field-row">
      <div>
        <div class="field-label">Pages</div>
        <div class="field-hint">Two puzzles per page</div>
      </div>
      <div class="stepper">
        <button
          type="button"
          class="stepper-btn"
          onclick={() => (pages = Math.max(1, pages - 1))}
          aria-label="Fewer pages"
        >−</button>
        <div class="stepper-value">{pages}</div>
        <button
          type="button"
          class="stepper-btn"
          onclick={() => (pages = Math.min(20, pages + 1))}
          aria-label="More pages"
        >+</button>
      </div>
    </div>

    <div class="divider"></div>

    <button
      type="button"
      class="btn-primary"
      disabled={busy}
      onclick={handlePrint}
    >
      <!-- Print icon -->
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
        <rect x="6" y="3.5" width="12" height="6" rx="1"/>
        <path d="M6 17H4.5A1.5 1.5 0 013 15.5v-5A1.5 1.5 0 014.5 9h15A1.5 1.5 0 0121 10.5v5A1.5 1.5 0 0119.5 17H18"/>
        <rect x="6" y="14" width="12" height="6.5" rx="1"/>
      </svg>
      {busy ? 'Generating…' : `Generate ${pages} ${pages === 1 ? 'page' : 'pages'}`}
    </button>

    <p style="margin-top:10px; font-size:12px; color:var(--muted); line-height:1.5;
              padding:10px; background:var(--paper); border-radius:10px;">
      Larger sizes and longer booklets take a moment to generate.
    </p>
  </div>
</div>
