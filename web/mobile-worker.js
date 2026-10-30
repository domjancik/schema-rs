"use strict";
// Only the disconnected pairing screen and its tools are cached, never scenes or API traffic.
const CACHE = "schema-mobile-offline-v3";
const OFFLINE = [
  "/offline.html",
  "/servers-core.js",
  "/servers-ui.js",
  "/servers.css",
  "/app-updates.js",
  "/vendor/qrcode.js",
  "/vendor/jsQR.js",
];
self.addEventListener("install", (event) =>
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(OFFLINE))),
);
self.addEventListener("message", (event) => {
  if (event.data?.type === "SCHEMA_ACTIVATE_UPDATE")
    event.waitUntil(self.skipWaiting());
});
self.addEventListener("activate", (event) =>
  event.waitUntil(
    Promise.all([
      caches
        .keys()
        .then((keys) =>
          Promise.all(
            keys
              .filter(
                (key) =>
                  key.startsWith("schema-mobile-offline-") && key !== CACHE,
              )
              .map((key) => caches.delete(key)),
          ),
        ),
      self.clients.claim(),
    ]),
  ),
);
self.addEventListener("fetch", (event) => {
  const request = event.request,
    url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin) return;
  if (
    request.mode === "navigate" &&
    ["/mobile", "/mobile.html"].includes(url.pathname)
  ) {
    event.respondWith(
      fetch(request).catch(() => caches.match("/offline.html")),
    );
    return;
  }
  if (OFFLINE.includes(url.pathname))
    event.respondWith(fetch(request).catch(() => caches.match(url.pathname)));
});
