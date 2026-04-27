declare global {
  interface Window {
    goatcounter?: { count: (opts: { path: string; event: boolean }) => void };
  }
}

export function trackEvent(name: string): void {
  // import.meta.env.PROD is true for every non-development build including
  // --mode test, so check MODE directly to stay in sync with vite.config.ts
  // which only injects the GoatCounter <script> when mode === 'production'.
  if (import.meta.env.MODE === 'production') {
    window.goatcounter?.count({ path: name, event: true });
  }
}
