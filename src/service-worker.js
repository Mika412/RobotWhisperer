/// <reference types="@sveltejs/kit" />
import { build, files, version } from "$service-worker";
import { toMeshAssetUrl } from "$lib/robotkit/meshAssetUrl";

const CACHE = `cache-${version}`;
const ASSETS = [...new Set([...build, ...files].map(toMeshAssetUrl))];

self.addEventListener("install", (event) => {
  async function addFilesToCache() {
    const cache = await caches.open(CACHE);
    await Promise.all(
      ASSETS.map(async (asset) => {
        const response = await fetch(asset).catch(() => null);
        if (response?.ok) await cache.put(asset, response);
      }),
    );
  }

  event.waitUntil(addFilesToCache());
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  async function takeOver() {
    for (const key of await caches.keys()) {
      if (key !== CACHE) await caches.delete(key);
    }
    await self.clients.claim();
  }

  event.waitUntil(takeOver());
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;

  async function respond() {
    const url = new URL(event.request.url);
    const cache = await caches.open(CACHE);

    if (ASSETS.includes(url.pathname)) {
      const cached = await cache.match(event.request);
      if (cached) return cached;
    }

    try {
      const response = await fetch(event.request);
      if (response.status === 200) {
        cache.put(event.request, response.clone());
      }
      return response;
    } catch {
      const cached = await cache.match(event.request);
      if (cached) return cached;
      throw new Error(`service worker: no response for ${url.pathname}`);
    }
  }

  event.respondWith(respond());
});
