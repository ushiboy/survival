/** The languages the page speaks. */
export const LOCALES = ['ja', 'en'] as const;
export type Locale = (typeof LOCALES)[number];

/**
 * The page's language for a browser language tag (`navigator.language`, e.g.
 * `ja-JP`): Japanese for Japanese, English for anything else.
 */
export function pickLocale(language: string | undefined): Locale {
  return language?.toLowerCase().split('-')[0] === 'ja' ? 'ja' : 'en';
}

/** The page's language for this browser. */
export function detectLocale(): Locale {
  return pickLocale(typeof navigator === 'undefined' ? undefined : navigator.language);
}
