declare global {
  interface Window {
    goatcounter?: { count: (opts: { path: string; event: boolean }) => void };
  }
}

export function trackEvent(name: string): void {
  if (import.meta.env.PROD) {
    window.goatcounter?.count({ path: name, event: true });
  }
}
