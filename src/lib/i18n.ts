export type Catalog = Record<string, string>;
export type Params = Record<string, string | number>;

export const FALLBACK_LOCALE = 'en';

/** First available locale matching the user's preferred languages (by primary subtag). */
export function detectLocale(preferred: readonly string[], available: readonly string[]): string {
  for (const tag of preferred) {
    const primary = tag.toLowerCase().split('-')[0];
    if (primary && available.includes(primary)) return primary;
  }
  return FALLBACK_LOCALE;
}

export function createTranslator(
  catalogs: Record<string, Catalog>,
  locale: string,
): (key: string, params?: Params) => string {
  return (key, params = {}) => {
    const message = catalogs[locale]?.[key] ?? catalogs[FALLBACK_LOCALE]?.[key] ?? key;
    return message.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
      name in params ? String(params[name]) : placeholder,
    );
  };
}

/** Keys a translation lacks, and keys it has that the reference does not. */
export function missingAndExtraKeys(
  reference: Catalog,
  candidate: Catalog,
): { missing: string[]; extra: string[] } {
  return {
    missing: Object.keys(reference).filter((key) => !(key in candidate)),
    extra: Object.keys(candidate).filter((key) => !(key in reference)),
  };
}
