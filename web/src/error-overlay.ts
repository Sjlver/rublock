// Defense-in-depth: any error that escapes a try/catch — a Rust panic that
// surfaces as a JS exception, an unhandled promise rejection, a programming
// bug — would otherwise leave the user staring at a frozen page. We listen
// once at startup and replace the screen with a user-visible banner so the
// failure mode is obvious and recoverable (refresh).

let overlayShown = false;

function describe(err: unknown): string {
  if (err instanceof Error) return err.message || err.toString();
  if (typeof err === 'string') return err;
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

function showOverlay(message: string): void {
  if (overlayShown) return;
  overlayShown = true;
  const el = document.createElement('div');
  el.setAttribute('role', 'alert');
  el.dataset.testid = 'error-overlay';
  el.style.cssText = [
    'position:fixed',
    'inset:0',
    'z-index:9999',
    'display:flex',
    'align-items:center',
    'justify-content:center',
    'padding:24px',
    'background:rgba(20,20,40,0.45)',
    'font-family:inherit',
  ].join(';');
  const card = document.createElement('div');
  card.style.cssText = [
    'max-width:420px',
    'background:#fff',
    'color:#222',
    'border-radius:14px',
    'padding:20px 22px',
    'box-shadow:0 6px 24px rgba(0,0,0,0.18)',
  ].join(';');
  const title = document.createElement('h2');
  title.textContent = 'Something went wrong';
  title.style.cssText = 'margin:0 0 8px;font-size:1.1rem;';
  const body = document.createElement('p');
  body.textContent = message;
  body.style.cssText = 'margin:0 0 14px;color:#444;white-space:pre-wrap;';
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.textContent = 'Reload';
  btn.style.cssText = [
    'padding:8px 14px',
    'border:0',
    'border-radius:8px',
    'background:#3346c8',
    'color:#fff',
    'font:inherit',
    'cursor:pointer',
  ].join(';');
  btn.addEventListener('click', () => window.location.reload());
  card.append(title, body, btn);
  el.appendChild(card);
  document.body.appendChild(el);
}

export function installErrorOverlay(): void {
  window.addEventListener('error', (ev) => {
    showOverlay(describe(ev.error ?? ev.message));
  });
  window.addEventListener('unhandledrejection', (ev) => {
    showOverlay(describe(ev.reason));
  });
}

export function reportFatal(err: unknown): void {
  showOverlay(describe(err));
}
