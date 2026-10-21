import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

async function start() {
  // Plain browser during development: fake backend. Tree-shaken out of production builds.
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) (await import('./lib/mock')).installMock();
  mount(App, { target: document.getElementById('app')! });
}

start();
