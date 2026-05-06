import { mount } from 'svelte';
import App from './components/App.svelte';
import { installErrorOverlay } from './error-overlay';
import './styles/global.css';
import './styles/puzzle.css';

installErrorOverlay();

const target = document.getElementById('app');
if (!target) throw new Error('#app mount point missing from index.html');

mount(App, { target });
