<script lang="ts">
  import { onMount } from 'svelte';

  // Build-time publisher id; unset → AdSlot renders nothing (ships safely
  // before EthicalAds approval). The MODE check mirrors `analytics.ts`
  // and the inject-goatcounter plugin: only `--mode production` enables.
  const PUBLISHER_ID = import.meta.env.VITE_ETHICALADS_PUBLISHER_ID as string | undefined;
  const ENABLED = import.meta.env.MODE === 'production' && !!PUBLISHER_ID;

  let mounted = $state(false);

  onMount(() => {
    if (!ENABLED) return;
    // Idempotent: the SPA mounts AdSlot once per solve, but the EthicalAds
    // client script only needs to load once across the session.
    if (!document.querySelector('script[data-ea-script]')) {
      const s = document.createElement('script');
      s.src = 'https://media.ethicalads.io/media/client/ethicalads.min.js';
      s.async = true;
      s.dataset.eaScript = '1';
      document.head.appendChild(s);
    }
    mounted = true;
  });
</script>

{#if ENABLED && mounted}
  <aside
    class="ad-slot"
    data-ad-slot
    data-ea-publisher={PUBLISHER_ID}
    data-ea-type="image"
    data-ea-keywords="puzzle|games|logic"
    data-ea-style="stickybox"
    aria-label="Advertisement"
  ></aside>
{/if}

<style>
  .ad-slot {
    display: block;
    margin: 1.25rem auto 0;
    max-width: 360px;
    /* Reserve vertical space so the ad doesn't cause CLS when it loads. */
    min-height: 100px;
  }
</style>
