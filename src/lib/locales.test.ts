import { describe, expect, it } from 'vitest';
import { catalogs } from './locales';
import { missingAndExtraKeys } from './i18n';

// Guards contributed translations: every locale must have exactly the keys of en.json.
describe('locale files', () => {
  const others = Object.entries(catalogs).filter(([locale]) => locale !== 'en');

  it('include English and French', () => {
    expect(Object.keys(catalogs)).toEqual(expect.arrayContaining(['en', 'fr']));
  });

  it.each(others)('%s has the same keys as en.json', (_locale, catalog) => {
    expect(missingAndExtraKeys(catalogs.en!, catalog)).toEqual({ missing: [], extra: [] });
  });

  it.each(Object.entries(catalogs))('%s has no empty message', (_locale, catalog) => {
    expect(Object.entries(catalog).filter(([, message]) => message.trim() === '')).toEqual([]);
  });
});
