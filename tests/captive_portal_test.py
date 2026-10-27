import importlib.util,pathlib,unittest,threading,http.client,urllib.parse,http.cookies,re,json,time
ROOT=pathlib.Path(__file__).resolve().parents[1]
class PortalTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  p=ROOT/'scripts/deploy/captive_portal.py'
  if not p.exists():return
  spec=importlib.util.spec_from_file_location('portal',p);cls.module=importlib.util.module_from_spec(spec);spec.loader.exec_module(cls.module)
 def setUp(self):
  self.assertTrue((ROOT/'scripts/deploy/captive_portal.py').exists(),'Captive login portal exists')
  class Backend(http.server.BaseHTTPRequestHandler):
   def do_GET(s):s.respond()
   def do_POST(s):s.respond()
   def respond(s):
    data=json.dumps({'path':s.path,'token':s.headers.get('x-schema-token'),'origin':s.headers.get('origin'),'cookie':s.headers.get('cookie')}).encode();s.send_response(200);s.send_header('Content-Length',len(data));s.end_headers();s.wfile.write(data)
   def log_message(*a):pass
  self.backend=http.server.ThreadingHTTPServer(('127.0.0.1',0),Backend);threading.Thread(target=self.backend.serve_forever,daemon=True).start()
  self.server=self.module.PortalServer(('127.0.0.1',0),{'schema_token':'test-backend-token-long-enough','admin_pin':'064219','allowed_hosts':['127.0.0.1','schema.local'],'upstream_port':self.backend.server_port})
  threading.Thread(target=self.server.serve_forever,daemon=True).start();self.host='127.0.0.1:'+str(self.server.server_port);self.origin='http://'+self.host
 def tearDown(self):
  for name in ['server','backend']:
   s=getattr(self,name,None)
   if s:s.shutdown();s.server_close()
 def request(self,method,path,body=None,headers=None):
  c=http.client.HTTPConnection('127.0.0.1',self.server.server_port,timeout=3);c.request(method,path,body,headers or {});r=c.getresponse();result=(r.status,dict(r.getheaders()),r.read());c.close();return result
 def challenge(self):
  code,h,b=self.request('GET','/login');self.assertEqual(code,200);self.assertNotIn(b'test-backend-token',b);nonce=re.search(b'name="nonce" value="([^"]+)"',b).group(1).decode();return nonce,h['Set-Cookie'].split(';')[0]
 def login(self,pin='064219',origin=None):
  nonce,cookie=self.challenge();return self.request('POST','/login',urllib.parse.urlencode({'pin':pin,'nonce':nonce}),{'Content-Type':'application/x-www-form-urlencoded','Cookie':cookie,'Origin':origin or self.origin})
 def test_probe_redirect_login_and_server_side_proxy(self):
  code,h,b=self.request('GET','/generate_204',headers={'Host':'connectivitycheck.gstatic.com'});self.assertEqual(code,302);self.assertEqual(h['Location'],self.origin+'/login')
  self.assertEqual(self.request('GET','/api/state')[0],401)
  code,h,b=self.login();self.assertEqual(code,303);cookie=h['Set-Cookie'].split(';')[0];self.assertIn('HttpOnly',h['Set-Cookie']);self.assertIn('SameSite=Strict',h['Set-Cookie'])
  code,h,b=self.request('GET','/api/state',headers={'Cookie':cookie,'x-schema-token':'spoof'});self.assertEqual(code,200);data=json.loads(b);self.assertEqual(data['token'],'test-backend-token-long-enough');self.assertIsNone(data['cookie'])
  self.assertEqual(self.request('POST','/api/command','{}',{'Cookie':cookie,'Origin':'http://evil.test','Content-Type':'application/json'})[0],403)
  code,_,b=self.request('POST','/api/command','{}',{'Cookie':cookie,'Origin':self.origin,'Content-Type':'application/json'});self.assertEqual(code,200);self.assertEqual(json.loads(b)['origin'],'http://127.0.0.1:'+str(self.backend.server_port))
 def test_csrf_wrong_pin_rate_limit_and_logout(self):
  self.assertEqual(self.login(origin='http://evil.test')[0],403)
  self.assertEqual(self.request('POST','/login','pin=064219&nonce=forged',{'Origin':self.origin,'Content-Type':'application/x-www-form-urlencoded'})[0],403)
  for _ in range(5):self.assertEqual(self.login('999999')[0],401)
  self.assertEqual(self.login()[0],429)
 def test_logout_expiry_and_bad_host(self):
  code,h,_=self.login();cookie=h['Set-Cookie'].split(';')[0]
  self.assertEqual(self.request('POST','/portal/logout','',{'Cookie':cookie,'Origin':self.origin})[0],303)
  self.assertEqual(self.request('GET','/api/state',headers={'Cookie':cookie})[0],401)
  self.assertEqual(self.request('GET','/api/state',headers={'Cookie':cookie,'Host':'evil.test'})[0],403)
 def test_malformed_nonce_and_duplicate_fields_do_not_authenticate(self):
  self.assertEqual(self.request('POST','/login',urllib.parse.urlencode({'nonce':'☃','pin':'064219'}),{'Origin':self.origin,'Content-Type':'application/x-www-form-urlencoded','Cookie':'schema_login=ascii'})[0],403)
  nonce,cookie=self.challenge()
  self.assertEqual(self.request('POST','/login',urllib.parse.urlencode([('nonce',nonce),('pin','064219'),('pin','999999')]),{'Cookie':cookie,'Origin':self.origin,'Content-Type':'application/x-www-form-urlencoded'})[0],400)
 def test_expired_session_and_backend_failure(self):
  code,h,_=self.login();cookie=h['Set-Cookie'].split(';')[0];token=cookie.split('=',1)[1]
  with self.server.auth.lock:self.server.auth.sessions[token]=('127.0.0.1',time.monotonic()-1)
  self.assertEqual(self.request('GET','/api/state',headers={'Cookie':cookie})[0],401)
  code,h,_=self.login();cookie=h['Set-Cookie'].split(';')[0]
  self.backend.shutdown();self.backend.server_close();self.backend=None
  self.assertEqual(self.request('GET','/api/state',headers={'Cookie':cookie})[0],502)
 def test_request_framing_and_config_restrictions(self):
  self.assertEqual(self.request('POST','/login','x',{'Transfer-Encoding':'chunked'})[0],400)
  self.assertEqual(self.request('POST','/login',headers={'Content-Length':'999999999'})[0],413)
  with self.assertRaises(ValueError):self.module.PortalServer(('127.0.0.1',0),{'schema_token':'short','admin_pin':'064219','allowed_hosts':['127.0.0.1']})
import http.server
if __name__=='__main__':unittest.main()
