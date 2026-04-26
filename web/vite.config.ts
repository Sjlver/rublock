import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import type { HtmlTagDescriptor } from 'vite';

// `base: './'` keeps asset URLs relative so the build works under any subpath
// (the site is served from `/rublock/` on GitHub Pages, but a plain `file://`
// open or a different base also Just Works).
export default defineConfig(({ mode }) => ({
  plugins: [
    svelte(),
    mode === 'production' && {
      name: 'inject-goatcounter',
      transformIndexHtml(): HtmlTagDescriptor[] {
        return [
          {
            tag: 'script',
            attrs: {
              'data-goatcounter': 'https://purpureus.goatcounter.com/count',
              async: true,
              src: '//gc.zgo.at/count.js',
            },
            injectTo: 'head',
          },
        ];
      },
    },
  ],
  base: './',
  build: {
    target: 'es2022',
    outDir: 'dist',
    emptyOutDir: true,
  },
}));
