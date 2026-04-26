<script lang="ts">
  interface Props {
    selectedSize: number;
    onPrint: (size: number, pages: number) => Promise<void>;
    busy: boolean;
  }

  let { selectedSize = $bindable(), onPrint, busy }: Props = $props();

  let pages = $state(4);

  async function handlePrint(): Promise<void> {
    const clamped = Math.max(1, Math.min(20, Number.isFinite(pages) ? pages : 4));
    await onPrint(selectedSize, clamped);
  }
</script>

<section class="tab-panel">
  <div class="panel-card">
    <div class="controls-row">
      <div class="field">
        <label for="print-size">Size</label>
        <select
          id="print-size"
          onchange={(e) =>
            (selectedSize = Number.parseInt((e.currentTarget as HTMLSelectElement).value, 10))}
        >
          <option value="5" selected={selectedSize === 5}>5 × 5</option>
          <option value="6" selected={selectedSize === 6}>6 × 6</option>
          <option value="7" selected={selectedSize === 7}>7 × 7</option>
          <option value="8" selected={selectedSize === 8}>8 × 8</option>
        </select>
      </div>
      <div class="field">
        <label for="inp-pages" class="label-text">Pages</label>
        <input id="inp-pages" class="narrow" type="number" min="1" max="20" bind:value={pages} />
      </div>
      <button class="btn-primary" type="button" disabled={busy} onclick={handlePrint}>Print</button>
    </div>
    <p class="hint">Larger puzzle sizes and page counts may take a moment to generate.</p>
  </div>
</section>
