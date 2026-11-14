import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach } from 'vitest';

beforeEach(() => {
  // jsdom has no layout: nothing is under any point unless a test places it (see dom.ts).
  document.elementFromPoint = () => null;
});

afterEach(() => {
  cleanup();
  window.history.replaceState(null, '', '/');
  for (const c of document.cookie.split('; ').filter(Boolean)) {
    document.cookie = `${c.split('=')[0]}=; max-age=0; path=/`;
  }
});
