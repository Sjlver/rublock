declare global {
  interface Window {
    goatcounter?: { count: (opts: { path: string; event: boolean }) => void };
  }
}

export function trackEvent(name: string): void {
  // Exclude localhost so tests and local preview runs don't skew numbers.
  if (import.meta.env.PROD && window.location.hostname !== 'localhost') {
    window.goatcounter?.count({ path: name, event: true });
  }
}
