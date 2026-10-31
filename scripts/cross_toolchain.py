"""Pinned workspace-local C compiler for the Windows-to-ARM64 TLS build."""
import hashlib,json,os,pathlib,subprocess,urllib.request,zipfile
VERSION='0.15.2'
SHA256='3a0ed1e8799a2f8ce2a6e6290a9ff22e6906f8227865911fb7ddedc3cc14cb0c'
URL='https://ziglang.org/download/0.15.2/zig-x86_64-windows-0.15.2.zip'
def extract_verified(archive,destination,expected):
 archive=pathlib.Path(archive);destination=pathlib.Path(destination).resolve()
 if hashlib.sha256(archive.read_bytes()).hexdigest()!=expected:raise ValueError('Cross compiler archive checksum differs')
 with zipfile.ZipFile(archive) as zipped:
  if sum(i.file_size for i in zipped.infolist())>1024*1024*1024:raise ValueError('Cross compiler archive exceeds size limit')
  for entry in zipped.infolist():
   target=(destination/entry.filename).resolve()
   if not target.is_relative_to(destination) or '\\' in entry.filename or (entry.external_attr>>16)&0o170000==0o120000:raise ValueError('Unsafe compiler archive path')
  zipped.extractall(destination)
def environment(root):
 if os.name!='nt':return{}
 root=pathlib.Path(root).resolve();root.mkdir(parents=True,exist_ok=True);install=root/('zig-'+VERSION);exe=install/('zig-x86_64-windows-'+VERSION)/'zig.exe';marker=install/'verified.json'
 if not exe.is_file() or not marker.is_file():
  archive=root/('zig-'+VERSION+'.zip')
  if not archive.exists() or hashlib.sha256(archive.read_bytes()).hexdigest()!=SHA256:
   temporary=archive.with_suffix('.download')
   with urllib.request.urlopen(URL,timeout=60) as source,temporary.open('wb') as output:
    total=0
    while block:=source.read(1024*1024):
     total+=len(block)
     if total>100*1024*1024:raise ValueError('Compiler download exceeds limit')
     output.write(block)
   if hashlib.sha256(temporary.read_bytes()).hexdigest()!=SHA256:raise ValueError('Cross compiler download checksum differs')
   os.replace(temporary,archive)
  extract_verified(archive,install,SHA256);marker.write_text(json.dumps({'exe':hashlib.sha256(exe.read_bytes()).hexdigest()}))
 if hashlib.sha256(exe.read_bytes()).hexdigest()!=json.loads(marker.read_text())['exe']:raise ValueError('Installed compiler checksum differs')
 compiler=root/'schema-aarch64-cc.cmd';archiver=root/'schema-aarch64-ar.cmd'
 compiler.write_text('@echo off\n"'+str(exe)+'" cc %* -target aarch64-linux-musl\n');archiver.write_text('@echo off\n"'+str(exe)+'" ar %*\n')
 return{'CC_aarch64_unknown_linux_musl':str(compiler),'AR_aarch64_unknown_linux_musl':str(archiver),'ZIG_GLOBAL_CACHE_DIR':str(root/'zig-cache')}
if __name__=='__main__':
 import argparse
 parser=argparse.ArgumentParser();parser.add_argument('root',type=pathlib.Path);args=parser.parse_args();print(json.dumps(environment(args.root)))
