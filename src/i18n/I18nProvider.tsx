import { useEffect, type ReactNode } from 'react';
import { IntlProvider } from 'use-intl';
import { detectLocale, type Locale } from './locale';
import en from './messages/en';
import ja from './messages/ja';

const MESSAGES = { ja, en } satisfies Record<Locale, typeof ja>;

declare module 'use-intl' {
  interface AppConfig {
    Locale: Locale;
    Messages: typeof ja;
  }
}

interface Props {
  /** The language to speak; by default the browser's (`navigator.language`). */
  locale?: Locale;
  children: ReactNode;
}

/** Gives the page its text in the user's language, and tags the document with it. */
export function I18nProvider({ locale = detectLocale(), children }: Props) {
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  return (
    <IntlProvider locale={locale} messages={MESSAGES[locale]}>
      {children}
    </IntlProvider>
  );
}
