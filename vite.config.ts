import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { serviceWorker } from './scripts/pwa';

export default defineConfig({
  // Where the page is served from. GitHub Pages serves a project site under
  // /<repository>/, so the deploy workflow sets BASE_PATH (e.g. /survival/).
  base: process.env.BASE_PATH || '/',
  plugins: [react(), serviceWorker()],
  server: { port: 5173 },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['src/test/setup.ts'],
    restoreMocks: true,
    unstubGlobals: true,
  },
});
