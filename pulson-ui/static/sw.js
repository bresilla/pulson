// Service Worker for Pulson PWA
const CACHE_NAME = 'pulson-v1';
const RUNTIME_CACHE = 'pulson-runtime-v1';

const urlsToCache = [
  '/',
  '/static/styles/base.css',
  '/static/styles/auth.css',
  '/static/styles/dashboard.css',
  '/static/styles/settings.css',
  '/static/styles/pulse_visualization.css',
  '/static/styles/inline_map.css',
  '/static/styles/mobile.css',
  '/static/styles/pwa.css',
  '/static/pulson_ui.js',
  '/static/pulson_ui_bg.wasm',
  '/static/logo.png',
  '/static/manifest.json',
  '/static/offline.html',
  '/static/pwa-manager.js'
];

// Install event - cache resources
self.addEventListener('install', event => {
  console.log('Service Worker installing...');
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => {
        console.log('Caching app shell');
        return cache.addAll(urlsToCache);
      })
      .then(() => {
        // Force the waiting service worker to become the active service worker
        return self.skipWaiting();
      })
  );
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
  console.log('Service Worker activating...');
  event.waitUntil(
    caches.keys().then(cacheNames => {
      return Promise.all(
        cacheNames.map(cacheName => {
          if (cacheName !== CACHE_NAME && cacheName !== RUNTIME_CACHE) {
            console.log('Deleting old cache:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    }).then(() => {
      // Take control of all pages
      return self.clients.claim();
    })
  );
});

// Fetch event - implement caching strategies
self.addEventListener('fetch', event => {
  // Skip cross-origin requests
  if (!event.request.url.startsWith(self.location.origin)) {
    return;
  }

  // Handle different types of requests
  if (event.request.destination === 'document') {
    // For navigation requests, try network first, fallback to cache
    event.respondWith(
      fetch(event.request)
        .then(response => {
          // Cache the new version
          const responseClone = response.clone();
          caches.open(RUNTIME_CACHE).then(cache => {
            cache.put(event.request, responseClone);
          });
          return response;
        })
        .catch(() => {
          // Fallback to cache, then to offline page
          return caches.match(event.request)
            .then(response => {
              return response || caches.match('/') || caches.match('/static/offline.html');
            });
        })
    );
  } else {
    // For other requests, try cache first, fallback to network
    event.respondWith(
      caches.match(event.request)
        .then(response => {
          if (response) {
            return response;
          }
          
          return fetch(event.request)
            .then(response => {
              // Don't cache non-successful responses
              if (!response || response.status !== 200 || response.type !== 'basic') {
                return response;
              }

              // Cache the response for future use
              const responseToCache = response.clone();
              caches.open(RUNTIME_CACHE)
                .then(cache => {
                  cache.put(event.request, responseToCache);
                });

              return response;
            });
        })
    );
  }
});

// Push event - handle push notifications
self.addEventListener('push', event => {
  const options = {
    body: event.data ? event.data.text() : 'New update available',
    icon: '/static/logo.png',
    badge: '/static/logo.png',
    vibrate: [100, 50, 100],
    data: {
      dateOfArrival: Date.now(),
      primaryKey: '2'
    },
    actions: [
      {
        action: 'explore',
        title: 'View Dashboard',
        icon: '/static/logo.png'
      },
      {
        action: 'close',
        title: 'Close notification',
        icon: '/static/logo.png'
      }
    ]
  };

  event.waitUntil(
    self.registration.showNotification('Pulson Alert', options)
  );
});

// Notification click event
self.addEventListener('notificationclick', event => {
  event.notification.close();

  if (event.action === 'explore') {
    // Open the app and navigate to dashboard
    event.waitUntil(
      clients.openWindow('/')
    );
  } else if (event.action === 'close') {
    // Just close the notification
    event.notification.close();
  } else {
    // Default action - open the app
    event.waitUntil(
      clients.openWindow('/')
    );
  }
});

// Background sync (for offline actions)
self.addEventListener('sync', event => {
  if (event.tag === 'background-sync') {
    event.waitUntil(doBackgroundSync());
  }
});

function doBackgroundSync() {
  // Handle any pending actions when back online
  return Promise.resolve();
}

// Message event - handle messages from the main thread
self.addEventListener('message', event => {
  if (event.data && event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});
