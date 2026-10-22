import '@fontsource-variable/valley-sans';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import Overlay from './Overlay.svelte';

async function start() {
  // Plain browser during development: fake backend. Tree-shaken out of production builds.
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) (await import('./lib/mock')).installMock();
  // The backend loads index.html#overlay in a second, transparent window for the dictation pill.
  const overlay = location.hash === '#overlay';
  if (overlay) document.documentElement.classList.add('overlay');
  mount(overlay ? Overlay : App, { target: document.getElementById('app')! });
}

start();
