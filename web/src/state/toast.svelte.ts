export type ToastTone = 'info' | 'success' | 'error';

export const toastState = $state({
  text: '',
  tone: 'info' as ToastTone,
});

let timer: ReturnType<typeof setTimeout> | null = null;
let seq = 0;

export function showToast(text: string, tone: ToastTone = 'info', durationMs = 2400): void {
  if (timer) clearTimeout(timer);
  timer = null;
  toastState.text = text;
  toastState.tone = tone;
  if (durationMs > 0) {
    const mine = ++seq;
    timer = setTimeout(() => {
      if (seq === mine) toastState.text = '';
    }, durationMs);
  } else {
    seq++;
  }
}

export function clearToast(): void {
  if (timer) clearTimeout(timer);
  timer = null;
  toastState.text = '';
  seq++;
}
