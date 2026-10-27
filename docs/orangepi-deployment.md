# Orange Pi Schéma appliance

This deployment targets the inspected Orange Pi Zero 2W / H618 running Armbian Debian 13 (aarch64). It installs the native Rust engine, its embedded Studio and schémaboy clients, an OLA Art-Net-to-Open-DMX bridge, and a private phone Wi-Fi access point. The Windows development studio is independent.

## Connections

- Phone: join the configured WPA2 network and open `http://10.42.0.1:8787/` or `/mobile` for schémaboy. `schema.lan` resolves locally on the AP.
- Wired Windows management: board `10.0.0.2/24`, Windows Ethernet `10.0.0.1/24`; existing IPv6 link-local SSH is retained. No Windows network settings are changed.
- Enter the generated Schéma API token through the client Connection control. No token is placed in a URL.
- AP is local control only; internet routing/NAT is not configured. A phone may ask whether to keep a network without internet access.

```text
phone -- WPA2 --> wlan0 10.42.0.1:8787 -- Schéma
PC ------------> eth0 10.0.0.2:8787       |
                                        Art-Net 127.0.0.1:6454, universe 0
                                        |
                                        OLA -> FTDI Open DMX USB -> fixture
```

The initial scene contains one UV COB at DMX address 1, with channel 1 intensity and channel 2 constant program byte 0. It starts dark with Value 0 and Art-Net disabled. Output may be enabled by the operator. Saved projects and the full session live in `/var/lib/schema`; the trusted checkpoint wrapper preserves the operator's output enabled/disabled choice and undo history across boots.

## Services

- `schema.service`: nonroot `schema` user; native engine and embedded web server on port 8787. SIGINT shutdown flushes the final session.
- `schema-clock.service`: invokes the existing DS3231 restore helper after legacy fake-hwclock loaders, before Schéma.
- `olad.service`: dedicated `olad` user, native FTDI driver at 30 Hz; HTTP administration disabled and RPC bound to loopback. Art-Net UDP 6454 accepts protocol ingress; the local engine sends to 127.0.0.1.
- `schema-ola-bridge.timer`: checks every 5 seconds and reconciles Art-Net input to universe 0 and the single FTDI output to universe 0. It discovers aliases/ports and does not restart OLA.
- `schema-ola-usb-refresh.service`: USB 0403:6001 insertion reloads OLA plugins once, since the FTDI plugin discovers devices on startup; the timer re-patches new aliases. Multiple FTDI adapters require an explicit `SCHEMA_FTDI_UID` in `/etc/schema/bridge.env`. Other USB DMX protocols need their own configuration.
- `schema-ap.service` and `schema-ap-dhcp.service`: 2.4 GHz channel 6, existing IL regulatory domain, DHCP/local DNS on wlan0 only. Netplan's generated station-mode WPA unit and the competing packaged hostapd unit are masked. Ethernet DHCP/IPv6 remain available alongside the management IPv4 address.

USB absence is a normal `waiting_for_open_dmx_usb` state in `/run/schema-ola/status.json`. Device presence and physical DMX behavior require the actual adapter and fixture; software loopback verification is not physical output verification.

## RTC preservation

Original `/boot/armbianEnv.txt`, DS3231 overlay source/binary, module/udev files, original services/timer and helper scripts are backed up with SHA256 records under `/var/backups/schema-deployment/`; `latest` identifies the backup directory. They are kept unchanged.

The inspected board initially rejected DS3231 time reads (`RTC_RD_TIME` returned EINVAL). Standard `hwclock --systohc --utc --noadjfile --rtc=/dev/rtc-ds3231` initialized it from authenticated host UTC. A later power cycle showed hardware UTC advancing correctly while legacy fake-hwclock loaders overwrote system time. The additive late restore service corrects this ordering without rewriting the original RTC configuration. The existing 15-minute RTC sync timer remains enabled; its helper writes only when NTP synchronization is confirmed.

## Secrets and administrator access

Generated AP credentials and the web API token are stored root-only in `/etc/schema/access.json`; the engine token is passed through root-only `/etc/schema/schema.env`. Do not commit either file. AP and API credentials are generated directly on the device in mode 0600 staging inside a mode 0700 directory; no Windows credential payload or existing project is transferred.

The Windows administrator helper uses the existing SSH key and strictly verifies the saved host key. With the user's explicit authorization, the entered sudo credential is retained in native Windows Credential Manager under `schema-rs/orangepi/sudo` (Generic, local-machine persistence for the current Windows account). The password is never written to project files, JSON, environment variables, command lines or logs. Password collection is local-only and stores the value before any network operation. The SSH helper then verifies the stored value with sudo before privileged deployment work. A rejected value returns an error; it does not open another prompt. Remove the Generic credential in Windows Credential Manager to forget it.

`scripts/deploy/remote_admin.py` is a device-specific Windows provisioning helper using Paramiko and native Win32 credential APIs. It connects through the existing wired IPv6 management endpoint/scope and host-key alias. Initial package downloads used a separate temporary loopback proxy carried through SSH. The administrator controller itself contains no proxy setup, and production services do not depend on the Windows computer. It accepts explicit JSON jobs on stdin and retains a verified reconnect path.

## Installation and rollback

`install-core.sh` validates the ARM64 ELF/hash/source manifest, installs an immutable release below `/opt/schema/releases/<source-commit>/`, and updates `/opt/schema/current`. Existing project files are preserved. `install-bridge.sh` configures OLA and waits for RPC discovery. `armbian-ap.sh` applies the isolated AP configuration and arms a 10-minute rollback before writing changes. Cancel the transient rollback timer/service only after confirming AP-ENABLED and intact wired SSH. The rollback stops the custom AP/DHCP services and restores the preserved netplan station configuration; it does not touch RTC files.

For manual recovery, run `/usr/local/sbin/schema-ap-rollback` as root over wired SSH. For a binary rollback, switch `/opt/schema/current` to a verified previous release and restart `schema.service`.

Verify enabled/active service states, actual hostapd AP-ENABLED status, DHCP binding, API authentication, RTC/system UTC epoch skew, unchanged original RTC hashes, and a changed boot ID after a warm reboot. A successful service start alone is not proof that a phone associated or a physical DMX fixture received data.

## Verified deployment

The ARM64 musl release from source commit `154ed41` ran on the board and served all 186 catalog nodes and embedded web assets. Its SHA256 is `04a41f9fba753bc39a2ef723a179b38281ad4c6d13bd8b9c67c55882c7768f7f`. Desktop and mobile browser checks passed against the wired URL with API token authentication.

A scratch engine, gated on no FTDI hardware being present, sent a UV value of 0.5 through Art-Net to OLA universe 0. OLA reported 512 slots with dimmer 128, program 0 and all other slots zero. The scratch scene was returned to Value 0; all 512 slots were verified zero before and after the scratch sender stopped. Production scene/history were unchanged.

An actual warm reboot changed boot ID from `e1634812-7852-45f9-9fbd-90cd3efca357` to `35a00b11-f24e-4a1b-8fea-af41622dc7cd`. Schéma, late RTC restoration, OLA, AP, DHCP and patch timer started automatically. The initial named project `uv-cob`, scene, output disabled state and undo/redo history were retained. All 14 original RTC/boot configuration hashes matched after reboot. System time and RTC agree within a second; they are approximately 13 seconds behind the Windows reference. Optional recalibration was not performed.

The USB DMX adapter was not connected during deployment, so physical DMX and fixture output remain unverified. The bridge is in its normal waiting-for-adapter state and will discover/re-patch the common FTDI adapter when attached. Actual phone association was not exercised by automation; AP-ENABLED, channel 6, address persistence and DHCP/DNS bindings were verified.

## Captive login and mDNS

`install-portal.sh` adds a separate nonroot Python portal on port 80 without restarting the lighting engine or changing its scene/history. Open `http://schema.local/` on wired Ethernet or the Schéma Wi-Fi; `http://10.42.0.1/` is the AP fallback. Avahi publishes IPv4 HTTP service discovery on eth0 and wlan0 while keeping the system hostname unchanged.

The portal uses a generated six-digit administrator PIN and short-lived, IP-bound browser sessions. The existing long engine token remains server-side in a systemd credential and direct port 8787 access still requires that token. The PIN is stored with the other root-only access details in `/etc/schema/access.json`; `/etc/schema/portal.json` is also mode 0600. No credentials are advertised through mDNS or placed in URLs. Local HTTP relies on the trusted WPA2/local network; it does not provide TLS protection against other trusted network peers.

AP DNS resolves connectivity-probe hostnames to the local portal, which redirects phones into login. This supports legacy HTTP captive-network detection; phone behavior varies, and the manual URL remains available. The AP remains local-only without internet routing. DHCP option 114 is deliberately absent because the standards-based CAPPORT API requires trusted HTTPS. HTTPS connections are not intercepted or impersonated.

`schema-portal.service` and `avahi-daemon.service` start on boot. The portal runs with only the bind-low-port capability and reads its configuration through systemd LoadCredential. The installer backs up prior DNS/Avahi/access configuration under `/var/backups/schema-portal/`; restoring that backup and stopping the portal reverses this addition. It does not alter RTC files, radio configuration, OLA or engine output settings.

### Connected USB verification

On the resumed deployment, USB ID 0403:6001 appeared as FT232R USB UART, serial `A47I442S`. OLA's native FTDI plugin initialized its interface and exposed device UID `13-A47I442S`, output port 1, patched to universe 0. Art-Net device `2-1` input port 0 is also patched to universe 0. This was verified after the user's board restart (boot ID `103d0ce5-9b20-4544-8a49-0710444cbdaa`). The reconciler reported `ready`. The operator's current scene at revision 2 had output disabled and OLA held an empty universe; no nonzero test was sent to the connected hardware. Physical fixture response still requires an intentional lighting test.

A subsequent power cycle on the USB-connected deployment caused DS3231 reads to return EINVAL again; both RTC restore services consequently failed, although all 14 original configuration hashes still matched. This points to an RTC validity/retention problem that needs hardware diagnosis; battery retention is not established. The system clock fell back to its saved software time. No clock recalibration or original RTC configuration change was performed during this follow-up. Daytime dimming requires a valid current clock.

Captive deployment checks confirmed the portal and Avahi enabled/active, generated PIN authentication and authenticated API proxy on-device, fixed HTTP probe redirects to `http://10.42.0.1/login`, and AP DNS probe resolution to 10.42.0.1. Avahi advertised HTTP on both interfaces, and an independent multicast query from Windows Ethernet received `schema.local` → 10.0.0.2; no IPv6 AAAA answer was published. The engine, OLA and hostapd process IDs stayed unchanged and project revision 2/output-disabled state was preserved. Native Windows HTTP name resolution timed out despite successful direct mDNS; the explicit IP URL remains a fallback. Phone captive-window behavior still needs actual client confirmation.
