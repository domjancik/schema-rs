#!/bin/bash
# Adds captive login/mDNS without restarting the engine, radio, OLA or clock.
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
[ "$(id -u)" = 0 ]
stage=${1:?Pass directory containing captive_portal.py and schema-portal.service}
command -v avahi-daemon >/dev/null
[ -r /etc/schema/access.json ]
backup=/var/backups/schema-portal/$(date -u +%Y%m%dT%H%M%SZ)
install -d -m700 "$backup"
files=(/etc/schema/dnsmasq.conf /etc/avahi/avahi-daemon.conf /etc/avahi/services/schema.service /etc/schema/portal.json /etc/schema/access.json /etc/systemd/system/schema-portal.service /usr/local/lib/schema/captive_portal.py)
portal_was_active=$(systemctl is-active schema-portal.service || true)
for file in "${files[@]}"; do
 [ ! -e "$file" ] || cp -a --parents "$file" "$backup/"
done
rollback() {
 status=$?
 trap - ERR
 systemctl stop schema-portal.service || true
 for file in "${files[@]}"; do
  if [ -e "$backup$file" ]; then cp -a "$backup$file" "$file"; else rm -f -- "$file"; fi
 done
 systemctl daemon-reload
 if [ "$portal_was_active" = active ]; then systemctl start schema-portal.service || true; else systemctl disable schema-portal.service 2>/dev/null || true; fi
 systemctl restart avahi-daemon.service schema-ap-dhcp.service || true
 echo "Portal installation failed; prior configuration restored from $backup" >&2
 exit "$status"
}
trap rollback ERR
if ! getent passwd schema-portal >/dev/null; then useradd --system --no-create-home --shell /usr/sbin/nologin schema-portal; fi
install -d -m755 /usr/local/lib/schema /etc/avahi/services
install -m644 "$stage/captive_portal.py" /usr/local/lib/schema/captive_portal.py
install -m644 "$stage/schema-portal.service" /etc/systemd/system/schema-portal.service
python3 - <<'PY'
import json,pathlib,secrets,os
root=pathlib.Path('/etc/schema');access=root/'access.json';c=json.loads(access.read_text())
pin=c.get('admin_pin')
if not isinstance(pin,str) or len(pin)!=6 or not pin.isascii() or not pin.isdigit():pin=f'{secrets.randbelow(1000000):06d}'
c['admin_pin']=pin
portal={'schema_token':c['schema_token'],'admin_pin':pin,'allowed_hosts':['schema.local','schema-orangepi.local','orangepi.local','schema.lan','10.42.0.1','10.0.0.2'],'canonical_origin':'http://10.42.0.1','upstream_host':'127.0.0.1','upstream_port':8787}
for path,data in [(root/'portal.json',portal),(access,c)]:
 temporary=path.with_suffix('.new');fd=os.open(temporary,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
 with os.fdopen(fd,'w') as out:json.dump(data,out,indent=2);out.write('\n')
 os.chmod(temporary,0o600);temporary.replace(path)
PY
cat > /etc/avahi/avahi-daemon.conf <<'EOF'
[server]
host-name=schema
domain-name=local
use-ipv4=yes
use-ipv6=no
allow-interfaces=eth0,wlan0
[wide-area]
enable-wide-area=no
[publish]
publish-aaaa-on-ipv4=no
publish-hinfo=no
publish-workstation=no
[reflector]
enable-reflector=no
EOF
cat > /etc/avahi/services/schema.service <<'EOF'
<?xml version="1.0" standalone='no'?>
<!DOCTYPE service-group SYSTEM "avahi-service.dtd">
<service-group><name>Schéma Studio</name><service><type>_http._tcp</type><port>80</port><txt-record>path=/</txt-record></service></service-group>
EOF
systemd-analyze verify /etc/systemd/system/schema-portal.service
systemctl daemon-reload
systemctl enable --now schema-portal.service
# Confirm portal before directing phone connectivity probes to it.
python3 - <<'PY'
import time,urllib.request
for attempt in range(30):
 try:
  r=urllib.request.urlopen(urllib.request.Request('http://127.0.0.1/login',headers={'Host':'10.42.0.1'}),timeout=2)
  assert r.status==200;break
 except Exception:
  if attempt==29:raise
  time.sleep(.2)
PY
# DNS interception is AP-only; keep all lease/router/range settings unchanged.
python3 - <<'PY'
import pathlib
p=pathlib.Path('/etc/schema/dnsmasq.conf');text=p.read_text()
text=text.replace('local=/schema.lan/','local=/#/').replace('address=/schema.lan/10.42.0.1','address=/#/10.42.0.1')
assert 'interface=wlan0' in text and 'listen-address=10.42.0.1' in text
p.write_text(text)
PY
dnsmasq --test --conf-file=/etc/schema/dnsmasq.conf
systemctl enable avahi-daemon.service
systemctl restart avahi-daemon.service
systemctl restart schema-ap-dhcp.service
trap - ERR
printf 'Portal/mDNS installed; original services not restarted. Backup: %s\n' "$backup"
