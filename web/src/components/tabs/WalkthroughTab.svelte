<script lang="ts">
  import PageHeader from '../PageHeader.svelte';

  // TODO: Implement step-by-step solve walkthrough.
  // This tab should show a selected puzzle being solved one deduction at a time,
  // with each step highlighted on the board and explained in text.
  // See DESIGN_NOTES.md for context.
</script>

<PageHeader title="Walkthrough" status="Coming soon" />

<div class="tab-content">
  <div class="walkthrough-placeholder">
    <!-- Steps icon -->
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
      style="opacity: 0.5"
    >
      <path d="M4 18h4v-4" />
      <path d="M10 14h4v-4" />
      <path d="M16 10h4V6" />
    </svg>
    <p style="margin-top: 10px;">Step-by-step solver — in planning.</p>
  </div>
</div>
