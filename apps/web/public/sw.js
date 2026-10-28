// Offline cache (T8.2, rule 9). The data bundle is cached by its SHA-256 (`?v=`), cache first;
// everything else from this origin is network first with the cache as fallback, so after one
// online load the app works with no network at all. No other origin is ever fetched.
const SHELL = "mushaf-shell-v1";
const DATA = "mushaf-data-v1";

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (url.origin !== location.origin || e.request.method !== "GET") return;
  if (url.pathname.endsWith(".bundle") && url.searchParams.has("v")) {
    e.respondWith(
      caches.open(DATA).then(async (c) => {
        const hit = await c.match(e.request);
        if (hit) return hit;
        const res = await fetch(e.request);
        if (res.ok) {
          for (const k of await c.keys()) if (new URL(k.url).pathname === url.pathname) await c.delete(k);
          await c.put(e.request, res.clone());
        }
        return res;
      }),
    );
    return;
  }
  e.respondWith(
    caches.open(SHELL).then(async (c) => {
      try {
        const res = await fetch(e.request);
        if (res.ok) await c.put(e.request, res.clone());
        return res;
      } catch (err) {
        const hit = await c.match(e.request, { ignoreSearch: url.pathname === "/" });
        if (hit) return hit;
        throw err;
      }
    }),
  );
});
