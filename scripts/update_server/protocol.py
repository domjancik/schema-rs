"""Shared, stdlib-only signed release protocol. Copied beside the root-owned Pi installer."""
import hashlib,json,re
FILES=('schema-rs','Schema-s-Windows-x64.zip','latest.json','SHA256SUMS.txt')
CHECKS=('fmt','tests','clippy','syntax','smoke')
ID=re.compile(r'^[a-f0-9]{12}-[a-f0-9]{12}-[1-9][0-9]{0,9}$')
def canonical(value):return (json.dumps(value,sort_keys=True,separators=(',',':'))+'\n').encode()
def digest(path):
 h=hashlib.sha256()
 with open(path,'rb') as source:
  for block in iter(lambda:source.read(1024*1024),b''):h.update(block)
 return h.hexdigest()
def validate_manifest(value):
 if value.get('schema')!=1 or value.get('target')!='aarch64-unknown-linux-musl' or not ID.fullmatch(value.get('release_id','')):raise ValueError('Invalid release protocol/id/target')
 if not re.fullmatch('[a-f0-9]{40}',value.get('source_commit','')) or not re.fullmatch('[a-f0-9]{64}',value.get('tree','')):raise ValueError('Invalid source identity')
 if type(value.get('sequence')) is not int or value['sequence']<1:raise ValueError('Invalid release sequence')
 if value['release_id']!=value['source_commit'][:12]+'-'+value.get('files',{}).get('schema-rs',{}).get('sha256','')[:12]+'-'+str(value['sequence']):raise ValueError('Release identity does not match content')
 if set(value.get('files',{}))!=set(FILES):raise ValueError('Incomplete release files')
 if not all(value.get('checks',{}).get(name) is True for name in CHECKS):raise ValueError('Required validation did not pass')
 for entry in value['files'].values():
  if type(entry.get('size'))is not int or not 0<entry['size']<=128*1024*1024 or not re.fullmatch('[a-f0-9]{64}',entry.get('sha256','')):raise ValueError('Invalid artifact size/digest')
 return value

def validate_files(folder,manifest):
 validate_manifest(manifest)
 for name in FILES:
  path=folder/name
  if path.is_symlink() or not path.is_file() or path.stat().st_size!=manifest['files'][name]['size'] or digest(path)!=manifest['files'][name]['sha256']:raise ValueError('Artifact hash mismatch: '+name)
 with open(folder/'schema-rs','rb')as binary:raw=binary.read(64)
 if raw[:6]!=b'\x7fELF\x02\x01' or int.from_bytes(raw[18:20],'little')!=183:raise ValueError('Engine is not Linux ARM64 ELF')
 desktop=json.loads((folder/'latest.json').read_bytes());archive=manifest['files']['Schema-s-Windows-x64.zip']
 if desktop.get('commit')!=manifest['source_commit'][:12]or desktop.get('sha256')!=archive['sha256']or desktop.get('bytes')!=archive['size']:raise ValueError('Desktop metadata does not match release')
 return manifest
