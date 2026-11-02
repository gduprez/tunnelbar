import { mount } from 'svelte';
import App from './App.svelte';
import { detectLocale } from './lib/i18n';
import { catalogs } from './lib/locales';
import './styles.css';

if (import.meta.env.DEV && location.search.includes('preview')) {
  (await import('./lib/preview')).installPreview();
}

const locale = detectLocale(navigator.languages, Object.keys(catalogs));

mount(App, { target: document.getElementById('app')!, props: { locale } });
