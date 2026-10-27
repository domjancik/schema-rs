#!/bin/bash
# Run as root on the inspected Orange Pi Zero 2W using systemd-networkd.
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
[ "$(id -u)" = 0 ] || { echo 'Run as root' >&2; exit 1; }
[ -d /sys/class/net/wlan0 ] && [ -d /sys/class/net/eth0 ]
credentials=${1:?Pass a private JSON file with ap_ssid, ap_password and schema_token}
cat > /usr/local/sbin/schema-ap-rollback <<'EOF'
#!/bin/bash
set -euo pipefail
systemctl disable --now schema-ap-dhcp.service schema-ap.service || true
rm -f /etc/systemd/network/05-schema-ap.network
systemctl unmask netplan-wpa-wlan0.service
networkctl reload
networkctl reconfigure wlan0
systemctl start netplan-wpa-wlan0.service || true
EOF
chmod 700 /usr/local/sbin/schema-ap-rollback
# The automatic rollback is cancelled only after AP and wired SSH are verified.
systemctl stop schema-ap-rollback.timer schema-ap-rollback.service 2>/dev/null || true
systemctl reset-failed schema-ap-rollback.timer schema-ap-rollback.service 2>/dev/null || true
systemd-run --collect --unit=schema-ap-rollback --on-active=10min /usr/local/sbin/schema-ap-rollback
install -d -m750 /etc/schema
python3 - "$credentials" <<'PY'
import json,pathlib,re,sys,os
c=json.loads(pathlib.Path(sys.argv[1]).read_text())
assert re.fullmatch(r'[A-Za-z0-9_-]{1,32}',c['ap_ssid'])
assert re.fullmatch(r'[A-Za-z0-9_-]{16,63}',c['ap_password'])
assert re.fullmatch(r'[A-Za-z0-9_-]{24,128}',c['schema_token'])
p=pathlib.Path('/etc/schema')
(p/'hostapd.conf').write_text('interface=wlan0\ndriver=nl80211\nctrl_interface=/run/schema-hostapd\nssid='+c['ap_ssid']+'\ncountry_code=IL\nieee80211d=1\nhw_mode=g\nchannel=6\nwmm_enabled=1\nieee80211n=1\nauth_algs=1\nwpa=2\nwpa_key_mgmt=WPA-PSK\nrsn_pairwise=CCMP\nwpa_passphrase='+c['ap_password']+'\n')
(p/'schema.env').write_text('SCHEMA_TOKEN='+c['schema_token']+'\n')
(p/'access.json').write_text(json.dumps(c,indent=2)+'\n')
for name in ['hostapd.conf','schema.env','access.json']:os.chmod(p/name,0o600)
PY
# Earlier than netplan's generated 10-* file; does not change the Ethernet match.
cat > /etc/systemd/network/05-schema-ap.network <<'EOF'
[Match]
Name=wlan0
[Link]
RequiredForOnline=no
[Network]
Address=10.42.0.1/24
DHCP=no
LinkLocalAddressing=ipv6
ConfigureWithoutCarrier=yes
IPv6AcceptRA=no
EOF
# Add a management address while retaining DHCP and IPv6 link-local Ethernet.
install -d -m755 /etc/systemd/network/10-netplan-all-eth-interfaces.network.d
cat > /etc/systemd/network/10-netplan-all-eth-interfaces.network.d/50-schema-management.conf <<'EOF'
[Network]
Address=10.0.0.2/24
EOF
if ! getent passwd schema-ap >/dev/null; then useradd --system --home-dir /var/lib/schema-ap --shell /usr/sbin/nologin schema-ap; fi
install -d -o schema-ap -g schema-ap -m750 /var/lib/schema-ap
cat > /etc/schema/dnsmasq.conf <<'EOF'
interface=wlan0
except-interface=lo
bind-interfaces
listen-address=10.42.0.1
no-resolv
no-hosts
local=/schema.lan/
address=/schema.lan/10.42.0.1
dhcp-range=10.42.0.20,10.42.0.200,255.255.255.0,12h
dhcp-option=3,10.42.0.1
dhcp-option=6,10.42.0.1
dhcp-authoritative
dhcp-leasefile=/var/lib/schema-ap/dnsmasq.leases
user=schema-ap
group=schema-ap
EOF
cat > /etc/systemd/system/schema-ap.service <<'EOF'
[Unit]
Description=Schéma phone Wi-Fi access point
After=sys-subsystem-net-devices-wlan0.device systemd-networkd.service
Wants=systemd-networkd.service
BindsTo=sys-subsystem-net-devices-wlan0.device
Conflicts=netplan-wpa-wlan0.service wpa_supplicant@wlan0.service
[Service]
Type=simple
ExecStartPre=/usr/bin/ip link set wlan0 up
ExecStartPre=/usr/bin/ip address replace 10.42.0.1/24 dev wlan0
ExecStart=/usr/sbin/hostapd /etc/schema/hostapd.conf
Restart=on-failure
RestartSec=3
RuntimeDirectory=schema-hostapd
RuntimeDirectoryMode=0750
[Install]
WantedBy=multi-user.target
EOF
cat > /etc/systemd/system/schema-ap-dhcp.service <<'EOF'
[Unit]
Description=Schéma AP DHCP and local DNS
Requires=schema-ap.service
After=schema-ap.service
[Service]
Type=simple
ExecStart=/usr/sbin/dnsmasq --keep-in-foreground --conf-file=/etc/schema/dnsmasq.conf
Restart=on-failure
RestartSec=3
[Install]
WantedBy=multi-user.target
EOF
systemctl mask --now netplan-wpa-wlan0.service
systemctl mask --now hostapd.service
systemctl daemon-reload
networkctl reload
networkctl reconfigure wlan0
ip address replace 10.0.0.2/24 dev eth0
/usr/sbin/dnsmasq --test --conf-file=/etc/schema/dnsmasq.conf
systemd-analyze verify /etc/systemd/system/schema-ap.service /etc/systemd/system/schema-ap-dhcp.service
systemctl enable schema-ap.service schema-ap-dhcp.service
systemctl restart schema-ap.service schema-ap-dhcp.service
systemctl is-active schema-ap.service schema-ap-dhcp.service
ip -brief address show wlan0
