import { describe, expect, it } from 'vitest';
import { createTranslator, detectLocale, missingAndExtraKeys } from './i18n';

const catalogs = {
  en: { 'panel.title': 'Tunnels', 'tunnel.port': 'Port {port}' },
  fr: { 'panel.title': 'Tunnels FR' },
};

describe('detectLocale', () => {
  it('picks the first preferred language that is available', () => {
    expect(detectLocale(['de-DE', 'fr-FR', 'en-US'], ['en', 'fr'])).toBe('fr');
  });

  it('matches on the primary subtag, case-insensitively', () => {
    expect(detectLocale(['FR-ca'], ['en', 'fr'])).toBe('fr');
  });

  it('falls back to English when nothing matches', () => {
    expect(detectLocale(['ja-JP'], ['en', 'fr'])).toBe('en');
    expect(detectLocale([], ['en', 'fr'])).toBe('en');
  });
});

describe('createTranslator', () => {
  it('returns the message of the active locale', () => {
    const t = createTranslator(catalogs, 'fr');
    expect(t('panel.title')).toBe('Tunnels FR');
  });

  it('falls back to English when the key is missing in the active locale', () => {
    const t = createTranslator(catalogs, 'fr');
    expect(t('tunnel.port', { port: 5432 })).toBe('Port 5432');
  });

  it('returns the key itself when no catalog has it', () => {
    const t = createTranslator(catalogs, 'fr');
    expect(t('unknown.key')).toBe('unknown.key');
  });

  it('replaces every occurrence of a parameter and leaves unknown ones untouched', () => {
    const t = createTranslator({ en: { k: '{a}-{a}-{b}' } }, 'en');
    expect(t('k', { a: 1 })).toBe('1-1-{b}');
  });
});

describe('missingAndExtraKeys', () => {
  it('reports keys missing from or unknown to the reference catalog', () => {
    expect(missingAndExtraKeys({ a: '', b: '' }, { b: '', c: '' })).toEqual({
      missing: ['a'],
      extra: ['c'],
    });
  });
});
