#!/bin/bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
stage=${1:?Pass staging directory}
release=${2:?Pass source commit}
[[ "$release" =~ ^[0-9a-f]{7,40}$ ]]
[ "$(id -u)" = 0 ]
python3 - "$stage" "$release" <<'PY'
import hashlib,json,pathlib,sys
stage=pathlib.Path(sys.argv[1]);m=json.loads((stage/'arm-build.json').read_text());binary=(stage/'schema-rs').read_bytes()
assert m['source_commit']==sys.argv[2]
assert hashlib.sha256(binary).hexdigest()==m['sha256']
assert binary[:4]==bytes([127,69,76,70]) and binary[18:20]==bytes([183,0])
print('Verified ARM64 release SHA256:',m['sha256'])
PY
if ! getent passwd schema >/dev/null; then useradd --system --home-dir /var/lib/schema --create-home --shell /usr/sbin/nologin schema; fi
install -d -m755 /opt/schema/releases/"$release"
if [ -f /opt/schema/releases/"$release"/schema-rs ]; then
 cmp -s "$stage/schema-rs" /opt/schema/releases/"$release"/schema-rs || { echo 'Immutable release already exists with different bytes' >&2; exit 1; }
else
 install -m755 "$stage/schema-rs" /opt/schema/releases/"$release"/schema-rs
fi
install -m644 "$stage/arm-build.json" /opt/schema/releases/"$release"/BUILD.json
if [ -e /opt/schema/current ] && [ ! -L /opt/schema/current ]; then echo '/opt/schema/current is not a symlink; refusing to replace' >&2; exit 1; fi
ln -sfn "releases/$release" /opt/schema/current
install -d -o schema -g schema -m750 /var/lib/schema
install -d -m750 /etc/schema
python3 - "$stage" <<'PY'
import json,os,pathlib,re,sys
stage=pathlib.Path(sys.argv[1]);c=json.loads((stage/'credentials.json').read_text())
assert re.fullmatch(r'[A-Za-z0-9_-]{24,128}',c['schema_token'])
config=pathlib.Path('/etc/schema');(config/'schema.env').write_text('SCHEMA_TOKEN='+c['schema_token']+chr(10));(config/'access.json').write_text(json.dumps(c,indent=2)+chr(10))
for name in ['schema.env','access.json']:os.chmod(config/name,0o600)
root=pathlib.Path('/var/lib/schema');layout=root/'registry/layouts/init.json'
if not layout.exists():
 p=json.loads((stage/'initial-project.json').read_text());assert not p['output']['enabled'];assert len(p['fixtures'])==1;assert p['fixtures'][0]['dmx_model']=='uv-cob';assert p['content']['uv-cob-1'][0]['root']['properties']['Value']==0
 for folder in ['registry/layouts','registry/content','registry/config','registry/stacks','resources','plugins']:(root/folder).mkdir(parents=True,exist_ok=True)
 layout.write_text(json.dumps({k:p[k] for k in ['version','name','fixtures','dmx_models','dimming']},indent=2))
 entries=[{'fixture':fixture,'layer':layer,'stack':stack} for fixture,layers in p['content'].items() for layer,stack in enumerate(layers)]
 entries += [{'fixture':'__director','layer':layer,'stack':stack} for layer,stack in enumerate(p['director'])]
 (root/'registry/content/init.json').write_text(json.dumps(entries,indent=2));(root/'registry/config/output.json').write_text(json.dumps(p['output'],indent=2))
PY
chown -R schema:schema /var/lib/schema
install -m755 "$stage/schema-start.sh" /usr/local/sbin/schema-start
install -m644 "$stage/schema.service" /etc/systemd/system/schema.service
install -m644 "$stage/schema-clock.service" /etc/systemd/system/schema-clock.service
systemctl daemon-reload
systemd-analyze verify /etc/systemd/system/schema-clock.service /etc/systemd/system/schema.service
systemctl enable --now schema-clock.service
printf 'CORE_INSTALLED\n'
