# Install schémaboy on a phone

Open your engine’s `/mobile` page, then choose **Install app** from the ••• menu. Android browsers can offer an Install app prompt. On iPhone/iPad use the browser Share menu → Add to Home Screen and enable Open as Web App if offered. An explicit in-page prompt is shown only when the browser provides one.

The manifest starts in `/mobile` with standalone display and the real Schéma app mark. Portrait and landscape use one viewport with safe-area padding, a single header and an overflow menu. Tool dialogs scroll independently.

Full PWA installation requires a trusted HTTPS connection (or localhost/loopback for local development). The Orange Pi’s current plain HTTP LAN/AP addresses do not meet that requirement on a phone. Some browsers offer a home-screen shortcut there; the installation dialog explains the limitation rather than promising an app prompt. A trusted HTTPS reverse proxy for the same engine origin is required for the full install flow. No certificates, public tunnels or phone trust settings are configured by this feature.

The installed controller does not install the Rust engine on the phone. It needs the host and its network to edit/view the live scene. Engine state, authenticated API responses, output buffers and commands are never cached or queued. The service worker is scoped to `/mobile` and caches only a disconnected fallback page; every live navigation uses the network. Reconnect to the engine to continue. Existing portal authentication still applies.

References: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable and https://web.dev/learn/pwa/installation.
