/**
 * Registers the service worker (dist/sw.js, written by `vite build`) so the
 * game can be installed and started offline. Waits for the page to load so
 * caching the build does not compete with starting the game.
 */
export function registerServiceWorker(url: string, nav: Navigator = navigator) {
  if (!('serviceWorker' in nav)) return;
  window.addEventListener(
    'load',
    () => {
      nav.serviceWorker.register(url).catch(() => {
        // Without it the game still runs; it just needs the network.
      });
    },
    { once: true },
  );
}
