const STATIC_CACHE = "kartoteka-static-v1";
const SHELL_CACHE = "kartoteka-shell-v1";
const SHELL_URL = "/";
const STATIC_EXTENSIONS = [
  ".css",
  ".js",
  ".wasm",
  ".json",
  ".png",
  ".svg",
  ".ico",
];
const BYPASS_PATHS = ["/api/", "/auth/", "/oauth/"];

function isBypassed(url, request) {
  if (request.method !== "GET") {
    return true;
  }

  return BYPASS_PATHS.some((path) => url.pathname.startsWith(path));
}

function isStaticAsset(url) {
  return STATIC_EXTENSIONS.some((ext) => url.pathname.endsWith(ext));
}

self.addEventListener("install", (event) => {
  self.skipWaiting();
  event.waitUntil(
    caches.open(SHELL_CACHE).then((cache) =>
      cache.addAll([
        SHELL_URL,
        "/manifest.json",
        "/icons/favicon.svg",
        "/icons/apple-touch-icon.png",
      ])
    )
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(
        keys
          .filter((key) => ![STATIC_CACHE, SHELL_CACHE].includes(key))
          .map((key) => caches.delete(key))
      );
      await self.clients.claim();
    })()
  );
});

self.addEventListener("message", (event) => {
  if (event.data === "skipWaiting") {
    self.skipWaiting();
  }
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);

  if (url.origin !== self.location.origin || isBypassed(url, request)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request)
        .then((response) => {
          const copy = response.clone();
          event.waitUntil(caches.open(SHELL_CACHE).then((cache) => cache.put(SHELL_URL, copy)));
          return response;
        })
        .catch(async () => {
          const cached = await caches.match(SHELL_URL);
          return cached || Response.error();
        })
    );
    return;
  }

  if (!isStaticAsset(url)) {
    return;
  }

  event.respondWith(
    caches.match(request).then((cached) => {
      const networkPromise = fetch(request)
        .then((response) => {
          const copy = response.clone();
          event.waitUntil(caches.open(STATIC_CACHE).then((cache) => cache.put(request, copy)));
          return response;
        })
        .catch(() => cached || Response.error());

      return cached || networkPromise;
    })
  );
});
