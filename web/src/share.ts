import { showToast } from './state/toast.svelte';

export async function shareUrl(url: string): Promise<void> {
  try {
    if (navigator.share && /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent)) {
      await navigator.share({ title: 'Doplo puzzle', text: 'Try this Doplo puzzle:', url });
      return;
    }
    await navigator.clipboard.writeText(url);
    showToast('Link copied to clipboard', 'success');
  } catch {
    showToast('Could not share this puzzle', 'error');
  }
}
