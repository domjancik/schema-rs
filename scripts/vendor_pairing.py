"""Vendor pinned QR implementations without executing package installation scripts."""
import base64,hashlib,json,pathlib,tarfile,urllib.request,io
ROOT=pathlib.Path(__file__).resolve().parents[1]
PACKAGES=[
 ('qrcode-generator','2.0.4','mZSiP6RnbHl4xL2Ap5HfkjLnmxfKcPWpWe/c+5XxCuetEenqmNFf1FH/ftXPCtFG5/TDobjsjz6sSNL0Sr8Z9g==','dist/qrcode.js','qrcode.js'),
 ('jsqr','1.4.0','dxLob7q65Xg2DvstYkRpkYtmKm2sPJ9oFhrhmudT1dZvNFFTlroai3AWSpLey/w5vMcLBXRgOJsbXpdN9HzU/A==','dist/jsQR.js','jsQR.js')]
def main():
 target=ROOT/'web/vendor';target.mkdir(exist_ok=True);metadata=[];licenses=[]
 for name,version,integrity,source,filename in PACKAGES:
  url=f'https://registry.npmjs.org/{name}/-/{name}-{version}.tgz'
  data=urllib.request.urlopen(url,timeout=30).read()
  if base64.b64encode(hashlib.sha512(data).digest()).decode()!=integrity:raise ValueError('Package integrity mismatch: '+name)
  with tarfile.open(fileobj=io.BytesIO(data)) as archive:
   content=archive.extractfile('package/'+source).read();(target/filename).write_bytes(content)
   if name=='jsqr':licenses.append('jsQR 1.4.0 — Apache-2.0\n'+archive.extractfile('package/LICENSE').read().decode())
   else:licenses.append('qrcode-generator 2.0.4 — MIT\nCopyright (c) 2009 Kazuhiko Arase\n\n'+MIT)
  metadata.append({'name':name,'version':version,'source':url,'integrity':'sha512-'+integrity,'file':filename,'sha256':hashlib.sha256(content).hexdigest()})
 (target/'QR-VERSIONS.json').write_text(json.dumps(metadata,indent=2)+'\n',encoding='utf8')
 (target/'QR-LICENSES.txt').write_text('\n\n'.join(licenses)+'\n',encoding='utf8')
MIT='Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.'
if __name__=='__main__':main()
