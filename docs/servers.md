# Servers and phone pairing

Open **Servers** from the Studio toolbar or schémaboy’s **⋯** menu. The published [Servers page](https://schema-s.vercel.app/connect) uses the same saved list as the installed app on that origin.

## Save and open

Enter an address such as `schema.local:8787`, `http://10.42.0.1`, or a trusted HTTPS host. Add a name and choose Studio or schémaboy, then Save server. Bare addresses default to HTTP. A pasted direct mobile link selects schémaboy. Edit renames/revises an entry; Remove forgets it on this device. Each canonical origin has one entry.

**Open Studio / Open schémaboy** opens the UI served by that host, in a separate window or browser tab. Its interface therefore matches its engine version. Browser/OS controls may appear when leaving an installed app’s origin. The original browser workspace stays open. The app does not proxy API requests, probe addresses, or enumerate the network. Connecting to an HTTP server follows the browser’s normal connection rules.

The list stores only name, origin and preferred view in localStorage. It belongs to the app origin and browser profile; a PWA installed from another host has a separate list. PINs and API tokens are entered on the destination host and are never copied into saved addresses or pairing codes. Changing a saved address adds that origin as a separate entry; renaming the existing origin updates it.

## Pair using QR

Every saved server has **QR**, **Copy link**, and **Download QR**. On a native host the dialog also offers **Share this server QR**. Codes contain the direct host UI URL, so a phone’s regular camera can open the server without visiting the public website first. For another device, use the server’s LAN address; localhost and127.0.0.1 refer to that device itself.

Inside the app, **Scan QR** requests the rear camera, or **Choose QR image** reads a local PNG/JPEG/WebP/GIF up to8MB. Scanning fills the form for review; it never opens or saves a host automatically. Save the reviewed address, then Open it. QR generation and decoding run locally using pinned vendored codecs; images are not uploaded. Camera tracks stop on close, cancellation, hiding the app, or navigation, including when a permission grant arrives late.

Camera access needs HTTPS or localhost and browser permission. On an ordinary HTTP LAN connection, use a QR image or type the address. Physical phone camera/OS permission behavior still requires device testing; automated tests use actual generated images and synthetic MediaStreams.

## App updates

In Servers → **App updates**, choose **Check for app updates**, then **Update app now** when offered. **Reload app** also loads the current interface from its origin. Online app resources are network-first. An updated service worker waits for explicit activation while the app is open; there is no forced mid-edit reload.

Before reload, the client blurs the edited field, waits for authored requests in this page and embedded previews, and explicitly saves a running browser engine session. Saved servers, named projects and undo/redo remain in their existing storage. Activation replaces only this app’s offline helper cache, preserving unrelated caches and IndexedDB. A failed save or update activation leaves the app open with an error.

A native server supplies the interface bundled with its own engine build. Reloading that interface does not install an engine update. On the public browser app, the update check compares the loaded build identifier with the published manifest.

After opening the installed phone app online once, its offline fallback retains the Servers screen and QR tools. You can retrieve saved addresses, pair from an image, generate a QR, or open another reachable host while the original origin is unavailable. Live editing still comes from the chosen host or the browser engine; scenes and API responses are not put into the service-worker cache.
