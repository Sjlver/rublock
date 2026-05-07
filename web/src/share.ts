import { showToast } from './state/toast.svelte';
import { t } from './i18n/index.svelte';

export async function shareUrl(url: string): Promise<void> {
  try {
    if (navigator.share && /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent)) {
      await navigator.share({ title: t('share_title'), text: t('share_text'), url });
      return;
    }
    await navigator.clipboard.writeText(url);
    showToast(t('toast_link_copied'), 'success');
  } catch {
    showToast(t('toast_share_failed'), 'error');
  }
}
