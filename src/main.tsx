import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { registerServiceWorker } from './game/serviceWorker';
import { I18nProvider } from './i18n/I18nProvider';
import './styles.css';

// Only builds have a service worker; the dev server serves files as they change.
if (import.meta.env.PROD) registerServiceWorker(`${import.meta.env.BASE_URL}sw.js`);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <I18nProvider>
      <App />
    </I18nProvider>
  </StrictMode>,
);
