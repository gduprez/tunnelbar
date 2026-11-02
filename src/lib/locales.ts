import type { Catalog } from './i18n';

// Every locales/<lang>.json file is picked up automatically.
const files = import.meta.glob<Catalog>('../../locales/*.json', { eager: true, import: 'default' });

export const catalogs: Record<string, Catalog> = Object.fromEntries(
  Object.entries(files).map(([path, catalog]) => [path.replace(/^.*\/(.+)\.json$/, '$1'), catalog]),
);
