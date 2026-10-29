import importlib.util,json,pathlib,tempfile,unittest,sys
ROOT=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('browser_build',ROOT/'scripts/build_browser.py');build=importlib.util.module_from_spec(spec);spec.loader.exec_module(build)
class BrowserPackage(unittest.TestCase):
 def test_browser_export_requires_real_bindings_and_emits_verified_public_manifest(self):
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t);wasm=p/'wasm';wasm.mkdir()
   with self.assertRaises(ValueError):build.export_browser(p/'out',wasm)
   (wasm/'schema_web.js').write_text('export default async function(){}');(wasm/'schema_web_bg.wasm').write_bytes(b'\0asm\x01\0\0\0')
   notices=p/'notices';notices.mkdir();(notices/'LICENSE.txt').write_text('Test notice');out=build.export_browser(p/'out',wasm,notices)
   self.assertIn('data-engine="browser"',(out/'studio.html').read_text(encoding='utf8'))
   manifest=json.loads((out/'browser-build.json').read_text())
   self.assertIn('wasm/schema_web_bg.wasm',manifest['files'])
   self.assertFalse(any('.session' in name or 'schema-project' in name for name in manifest['files']))
 def test_existing_site_policy_is_not_broadened_for_wasm(self):
  config=json.loads((ROOT/'scripts/publication/vercel.json').read_text())
  policy=next(h['value'] for h in config['headers'][0]['headers'] if h['key']=='Content-Security-Policy')
  self.assertNotIn('wasm-unsafe-eval',policy);self.assertIn("frame-ancestors 'none'",policy)
  worker=next(g for g in config['headers'] if g['source']=='/browser-worker.mjs')
  value=next(h['value'] for h in worker['headers'] if h['key']=='Content-Security-Policy')
  self.assertIn("script-src 'self' 'wasm-unsafe-eval'",value);self.assertNotIn("'unsafe-eval'",value);self.assertIn("connect-src 'self'",value)
if __name__=='__main__':unittest.main()
