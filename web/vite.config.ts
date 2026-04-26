import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// `base: './'` keeps asset URLs relative so the build works under any subpath
// (the site is served from `/rublock/` on GitHub Pages, but a plain `file://`
// open or a different base also Just Works).
export default defineConfig({
  plugins: [svelte()],
  base: './',
  build: {
    target: 'es2022',
    outDir: 'dist',
    emptyOutDir: true,
  },
});
