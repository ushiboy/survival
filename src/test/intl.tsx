import { render, type RenderOptions } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { I18nProvider } from '../i18n/I18nProvider';
import type { Locale } from '../i18n/locale';

/** Renders `ui` with the page's text in `locale` (Japanese unless said: the tests read the Japanese). */
export function renderIn(ui: ReactElement, locale: Locale = 'ja', options?: Omit<RenderOptions, 'wrapper'>) {
  const wrapper = ({ children }: { children: ReactNode }) => <I18nProvider locale={locale}>{children}</I18nProvider>;
  return render(ui, { wrapper, ...options });
}
