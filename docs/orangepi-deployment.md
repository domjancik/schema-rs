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
- `olad.service`: dedicated `olad` user, native FTDI driver at 30 Hz; HTTP/RPC administration bound to loopback.
- `schema-ola-bridge.timer`: checks every 5 seconds and reconciles Art-Net input to universe 0 and the single FTDI output to universe 0. It discovers aliases/ports and does not restart OLA.
- `schema-ola-usb-refresh.service`: USB 0403:6001 insertion reloads OLA plugins once, since the FTDI plugin discovers devices on startup; the timer re-patches new aliases. Multiple FTDI adapters require an explicit `SCHEMA_FTDI_UID` in `/etc/schema/bridge.env`. Other USB DMX protocols need their own configuration.
- `schema-ap.service` and `schema-ap-dhcp.service`: 2.4 GHz channel 6, existing IL regulatory domain, DHCP/local DNS on wlan0 only. Netplan's generated station-mode WPA unit and the competing packaged hostapd unit are masked. Ethernet DHCP/IPv6 remain available alongside the management IPv4 address.

USB absence is a normal `waiting_for_open_dmx_usb` state in `/run/schema-ola/status.json`. Device presence and physical DMX behavior require the actual adapter and fixture; software loopback verification is not physical output verification.

## RTC preservation

Original `/boot/armbianEnv.txt`, DS3231 overlay source/binary, module/udev files, original services/timer and helper scripts are backed up with SHA256 records under `/var/backups/schema-deployment/`; `latest` identifies the backup directory. They are kept unchanged.

The inspected board initially rejected DS3231 time reads (`RTC_RD_TIME` returned EINVAL). Standard `hwclock --systohc --utc --noadjfile --rtc=/dev/rtc-ds3231` initialized it from authenticated host UTC. A later power cycle showed hardware UTC advancing correctly while legacy fake-hwclock loaders overwrote system time. The additive late restore service corrects this ordering without rewriting the original RTC configuration. The existing 15-minute RTC sync timer remains enabled; its helper writes only when NTP synchronization is confirmed.

## Secrets and administrator access

Generated AP credentials and the web API token are stored root-only in `/etc/schema/access.json`; the engine token is passed through root-only `/etc/schema/schema.env`. Do not commit either file. The provisioning staging directory is mode 0700 and credentials are uploaded mode 0600.

The Windows administrator helper uses the existing SSH key and strictly verifies the saved host key. With the user's explicit authorization, the entered sudo credential is retained in native Windows Credential Manager under `schema-rs/orangepi/sudo` (Generic, local-machine persistence for the current Windows account). The password is never written to project files, JSON, environment variables, command lines or logs. Password collection is local-only and stores the value before any network operation. The SSH helper then verifies the stored value with sudo before privileged deployment work. A rejected value returns an error; it does not open another prompt. Remove the Generic credential in Windows Credential Manager to forget it.

`scripts/deploy/remote_admin.py` is a device-specific Windows provisioning helper using Paramiko and native Win32 credential APIs. It connects through the existing wired IPv6 management endpoint/scope and host-key alias. Initial package downloads used a separate temporary loopback proxy carried through SSH. The administrator controller itself contains no proxy setup, and production services do not depend on the Windows computer. It accepts explicit JSON jobs on stdin and retains a verified reconnect path.

## Installation and rollback

`install-core.sh` validates the ARM64 ELF/hash/source manifest, installs an immutable release below `/opt/schema/releases/<source-commit>/`, and updates `/opt/schema/current`. Existing project files are preserved. `install-bridge.sh` configures OLA and waits for RPC discovery. `armbian-ap.sh` applies the isolated AP configuration and arms a 10-minute rollback before writing changes. Cancel the transient rollback timer/service only after confirming AP-ENABLED and intact wired SSH. The rollback stops the custom AP/DHCP services and restores the preserved netplan station configuration; it does not touch RTC files.

For manual recovery, run `/usr/local/sbin/schema-ap-rollback` as root over wired SSH. For a binary rollback, switch `/opt/schema/current` to a verified previous release and restart `schema.service`.

Verify enabled/active service states, actual hostapd AP-ENABLED status, DHCP binding, API authentication, RTC/system UTC epoch skew, unchanged original RTC hashes, and a changed boot ID after a warm reboot. A successful service start alone is not proof that a phone associated or a physical DMX fixture received data.
