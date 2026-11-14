// The service worker that lets the installed game start without a network.
// `vite build` (scripts/pwa.ts) fills in VERSION and PRECACHE and writes it
// out as dist/sw.js; it is not used by the dev server.
//
// - Install: cache the page and every file of this build.
// - The page itself: network first (so a new build shows up when online),
//   the cached copy when offline. Other files: the cache first (their names
//   carry a hash, so a cached one never goes stale).
// - A new build takes over at once and drops the old builds' caches.

const VERSION = '__VERSION__';
const PRECACHE = __PRECACHE__;
const PREFIX = 'castaway-island-';
const CACHE = PREFIX + VERSION;

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith(PREFIX) && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET' || new URL(req.url).origin !== self.location.origin) return;
  if (req.mode === 'navigate') {
    event.respondWith(
      fetch(req)
        .then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(CACHE).then((cache) => cache.put('./', copy));
          }
          return res;
        })
        .catch(() => caches.match('./', { cacheName: CACHE })),
    );
    return;
  }
  // ignoreVary: the server may answer with "Vary: Origin", and module scripts
  // are requested with an Origin header that the precache requests lacked.
  event.respondWith(caches.match(req, { ignoreSearch: true, ignoreVary: true }).then((hit) => hit || fetch(req)));
});
