#!/bin/bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
stage=${1:?Pass staging directory}
[ "$(id -u)" = 0 ]
getent passwd schema >/dev/null
getent passwd olad >/dev/null
systemctl stop olad.service || true
if pgrep -x olad >/dev/null; then echo 'An OLA daemon is still running; refusing a second instance' >&2; exit 1; fi
install -d -o olad -g olad -m2770 /etc/ola
# OLA writes default preference files. Disable unrelated transports for this bridge.
python3 - <<'PY'
import pathlib,re
root=pathlib.Path('/etc/ola')
for p in root.glob('ola-*.conf'):
 if p.name in ['ola-artnet.conf','ola-ftdidmx.conf','ola-universe.conf','ola-server.conf']:continue
 text=p.read_text();text=re.sub(r'^enabled\s*=.*$', 'enabled = false',text,flags=re.M)
 if not re.search(r'^enabled\s*=',text,flags=re.M):text+='\nenabled = false\n'
 p.write_text(text)
for name in ['opendmx','usbserial','usbdmx','dummy']:(root/f'ola-{name}.conf').write_text('enabled = false\n')
(root/'ola-artnet.conf').write_text('enabled = true\nip = 127.0.0.1\nuse_loopback = true\nnet = 0\nsubnet = 0\nshort_name = Schema USB bridge\nlong_name = Schema Art-Net to Open DMX USB\n')
(root/'ola-ftdidmx.conf').write_text('enabled = true\nfrequency = 30\n')
PY
chown olad:olad /etc/ola/*.conf
cat > /etc/systemd/system/olad.service <<'EOF'
[Unit]
Description=OLA loopback Art-Net to FTDI Open DMX bridge
After=network.target
[Service]
Type=simple
User=olad
Group=olad
SupplementaryGroups=plugdev dialout
ExecStart=/usr/bin/olad --config-dir /etc/ola --interface 127.0.0.1 --no-register-with-dns-sd --no-http-quit --log-level 3 --pid-location /run/olad
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=3
NoNewPrivileges=yes
ProtectHome=yes
ProtectSystem=strict
ReadWritePaths=/etc/ola /run/olad
RuntimeDirectory=olad
RuntimeDirectoryMode=0750
PrivateTmp=yes
[Install]
WantedBy=multi-user.target
EOF
install -m755 "$stage/ola_reconcile.py" /usr/local/sbin/schema-ola-reconcile
install -m644 "$stage/schema-ola-bridge.service" /etc/systemd/system/schema-ola-bridge.service
install -m644 "$stage/schema-ola-bridge.timer" /etc/systemd/system/schema-ola-bridge.timer
install -m644 "$stage/schema-ola-usb-refresh.service" /etc/systemd/system/schema-ola-usb-refresh.service
install -m644 "$stage/70-schema-opendmx.rules" /etc/udev/rules.d/70-schema-opendmx.rules
udevadm control --reload-rules
systemctl daemon-reload
systemd-analyze verify /etc/systemd/system/olad.service /etc/systemd/system/schema-ola-bridge.service /etc/systemd/system/schema-ola-bridge.timer /etc/systemd/system/schema-ola-usb-refresh.service
python3 - <<'PY'
from ola.OlaClient import Plugin
assert Plugin.OLA_PLUGIN_ARTNET==2 and Plugin.OLA_PLUGIN_FTDIDMX==13
PY
systemctl enable --now olad.service schema-ola-bridge.timer
python3 - <<'PY'
import socket,time
for attempt in range(30):
 try:
  with socket.create_connection(('127.0.0.1',9010),timeout=1):pass
  break
 except OSError:time.sleep(1)
else:raise SystemExit('OLA RPC did not become ready')
PY
for attempt in 1 2 3 4 5; do
 if systemctl start schema-ola-bridge.service; then break; fi
 sleep 2
done
systemctl show schema-ola-bridge.service -p Result -p ExecMainStatus
[ "$(systemctl show schema-ola-bridge.service -p Result --value)" = success ]
systemctl is-active olad.service schema-ola-bridge.timer
cat /run/schema-ola/status.json
