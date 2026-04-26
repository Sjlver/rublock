<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    onDone?: () => void;
  }

  let { onDone }: Props = $props();

  const EMOJIS = ['🎉', '✨', '🌟', '⭐', '💯', '🎊', '🥳', '🏆', '🔢'];
  const EMOJI_DENSITY = 50 / 1440; // emojis per pixel of screen width

  let container: HTMLDivElement;

  onMount(() => {
    const count = Math.max(1, Math.round(window.innerWidth * EMOJI_DENSITY));
    const timers: number[] = [];
    for (let i = 0; i < count; i++) {
      timers.push(
        window.setTimeout(() => {
          const el = document.createElement('span');
          el.className = 'emoji-drop';
          el.textContent = EMOJIS[Math.floor(Math.random() * EMOJIS.length)];
          el.style.left = `${Math.random() * 100}%`;
          const dur = (1.5 + Math.random() * 2).toFixed(2);
          el.style.setProperty('--spin', `${(Math.random() * 720 - 360).toFixed(0)}deg`);
          el.style.animation = `emoji-fall ${dur}s ease-in forwards`;
          container.appendChild(el);
          el.addEventListener('animationend', () => el.remove(), { once: true });
        }, Math.random() * 1200)
      );
    }
    const cleanup = window.setTimeout(() => onDone?.(), 5000);
    return () => {
      for (const t of timers) clearTimeout(t);
      clearTimeout(cleanup);
    };
  });
</script>

<div id="emoji-rain" bind:this={container}></div>
