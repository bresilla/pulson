// PWA Installation and Update Management
class PWAManager {
  constructor() {
    this.deferredPrompt = null;
    this.installButton = null;
    this.updateButton = null;
    this.init();
  }

  init() {
    this.setupInstallPrompt();
    this.setupUpdateNotification();
    this.setupPushNotifications();
  }

  setupInstallPrompt() {
    // Listen for the beforeinstallprompt event
    window.addEventListener('beforeinstallprompt', (e) => {
      console.log('PWA install prompt available');
      // Prevent the mini-infobar from appearing on mobile
      e.preventDefault();
      // Stash the event so it can be triggered later
      this.deferredPrompt = e;
      // Show install button
      this.showInstallButton();
    });

    // Listen for the app installed event
    window.addEventListener('appinstalled', (e) => {
      console.log('PWA was installed');
      this.hideInstallButton();
      // Clear the deferredPrompt
      this.deferredPrompt = null;
    });
  }

  setupUpdateNotification() {
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker.addEventListener('controllerchange', () => {
        console.log('New service worker activated');
        this.showUpdateButton();
      });

      // Check for updates
      navigator.serviceWorker.ready.then((registration) => {
        registration.addEventListener('updatefound', () => {
          const newWorker = registration.installing;
          newWorker.addEventListener('statechange', () => {
            if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
              console.log('New content is available');
              this.showUpdateButton();
            }
          });
        });
      });
    }
  }

  setupPushNotifications() {
    // Check if push notifications are supported
    if ('PushManager' in window && 'serviceWorker' in navigator) {
      // Request permission for notifications
      this.requestNotificationPermission();
    }
  }

  async requestNotificationPermission() {
    if (Notification.permission === 'default') {
      const permission = await Notification.requestPermission();
      console.log('Notification permission:', permission);
    }
  }

  showInstallButton() {
    // Create install button if it doesn't exist
    if (!this.installButton) {
      this.installButton = this.createInstallButton();
      document.body.appendChild(this.installButton);
    }
    this.installButton.style.display = 'block';
  }

  hideInstallButton() {
    if (this.installButton) {
      this.installButton.style.display = 'none';
    }
  }

  showUpdateButton() {
    // Create update button if it doesn't exist
    if (!this.updateButton) {
      this.updateButton = this.createUpdateButton();
      document.body.appendChild(this.updateButton);
    }
    this.updateButton.style.display = 'block';
  }

  hideUpdateButton() {
    if (this.updateButton) {
      this.updateButton.style.display = 'none';
    }
  }

  createInstallButton() {
    const button = document.createElement('button');
    button.innerHTML = '📱 Install App';
    button.className = 'pwa-install-button';
    button.style.cssText = `
      position: fixed;
      bottom: 20px;
      right: 20px;
      background: #2563eb;
      color: white;
      border: none;
      padding: 12px 20px;
      border-radius: 25px;
      cursor: pointer;
      font-size: 14px;
      box-shadow: 0 4px 12px rgba(37, 99, 235, 0.3);
      z-index: 1000;
      display: none;
      transition: all 0.3s ease;
    `;

    button.addEventListener('click', () => {
      this.installApp();
    });

    button.addEventListener('mouseenter', () => {
      button.style.transform = 'translateY(-2px)';
      button.style.boxShadow = '0 6px 16px rgba(37, 99, 235, 0.4)';
    });

    button.addEventListener('mouseleave', () => {
      button.style.transform = 'translateY(0)';
      button.style.boxShadow = '0 4px 12px rgba(37, 99, 235, 0.3)';
    });

    return button;
  }

  createUpdateButton() {
    const button = document.createElement('button');
    button.innerHTML = '🔄 Update Available';
    button.className = 'pwa-update-button';
    button.style.cssText = `
      position: fixed;
      top: 20px;
      right: 20px;
      background: #059669;
      color: white;
      border: none;
      padding: 12px 20px;
      border-radius: 25px;
      cursor: pointer;
      font-size: 14px;
      box-shadow: 0 4px 12px rgba(5, 150, 105, 0.3);
      z-index: 1000;
      display: none;
      transition: all 0.3s ease;
    `;

    button.addEventListener('click', () => {
      this.updateApp();
    });

    return button;
  }

  async installApp() {
    if (this.deferredPrompt) {
      // Show the install prompt
      this.deferredPrompt.prompt();
      // Wait for the user to respond to the prompt
      const { outcome } = await this.deferredPrompt.userChoice;
      console.log(`User response to the install prompt: ${outcome}`);
      // Clear the saved prompt since it can't be used again
      this.deferredPrompt = null;
      this.hideInstallButton();
    }
  }

  updateApp() {
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker.ready.then((registration) => {
        if (registration.waiting) {
          // Tell the waiting service worker to skip waiting and become active
          registration.waiting.postMessage({ type: 'SKIP_WAITING' });
        }
      });
    }
    this.hideUpdateButton();
    // Reload the page to use the new service worker
    window.location.reload();
  }

  // Check if app is running as PWA
  isRunningAsPWA() {
    return window.matchMedia('(display-mode: standalone)').matches ||
           window.navigator.standalone === true;
  }

  // Show app-like behavior when running as PWA
  setupPWABehavior() {
    if (this.isRunningAsPWA()) {
      console.log('Running as PWA');
      // Add PWA-specific styles or behavior
      document.body.classList.add('pwa-mode');
    }
  }
}

// Initialize PWA manager when DOM is loaded
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => {
    window.pwaManager = new PWAManager();
  });
} else {
  window.pwaManager = new PWAManager();
}

export default PWAManager;
