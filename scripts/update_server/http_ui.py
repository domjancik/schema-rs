"""Loopback-only operator GUI; mutating requests require same-origin session+CSRF."""
import http.cookies,http.server,json,secrets,threading,time,urllib.parse,pathlib
class Server(http.server.ThreadingHTTPServer):
 daemon_threads=True
 def __init__(self,address,manager):
  if address[0]!='127.0.0.1':raise ValueError('Update GUI must bind loopback')
  self.manager=manager;self.sessions={};self.session_lock=threading.Lock();self.slots=threading.BoundedSemaphore(16);super().__init__(address,Handler)
 def process_request_thread(self,request,address):
  if not self.slots.acquire(False):request.close();return
  try:super().process_request_thread(request,address)
  finally:self.slots.release()
class Handler(http.server.BaseHTTPRequestHandler):
 protocol_version='HTTP/1.1'
 def setup(self):super().setup();self.connection.settimeout(8)
 def log_message(self,*args):pass
 def host(self):return self.headers.get('Host')in('127.0.0.1:'+str(self.server.server_port),'localhost:'+str(self.server.server_port))
 def session(self,create=False):
  cookie=http.cookies.SimpleCookie()
  try:cookie.load(self.headers.get('Cookie',''))
  except http.cookies.CookieError:return None,None
  key=cookie.get('schema_updater');key=key.value if key else None
  with self.server.session_lock:
   now=time.monotonic();self.server.sessions={k:v for k,v in self.server.sessions.items()if v[1]>now};value=self.server.sessions.get(key)
   if value:return key,value[0]
   if not create or len(self.server.sessions)>=128:return None,None
   key=secrets.token_urlsafe(32);csrf=secrets.token_urlsafe(32);self.server.sessions[key]=(csrf,now+28800);return key,csrf
 def respond(self,status,body,mime='application/json',cookie=None):
  data=body if isinstance(body,bytes)else json.dumps(body).encode();self.send_response(status);self.send_header('Content-Type',mime);self.send_header('Content-Length',str(len(data)));self.send_header('Cache-Control','no-store');self.send_header('X-Content-Type-Options','nosniff');self.send_header('Content-Security-Policy',"default-src 'self'; script-src 'self'; style-src 'self'; frame-ancestors 'none'; base-uri 'none'")
  if cookie:self.send_header('Set-Cookie','schema_updater='+cookie+'; Path=/; HttpOnly; SameSite=Strict; Max-Age=28800')
  self.end_headers();self.wfile.write(data)
 def do_GET(self):
  if not self.host()or self.headers.get('Sec-Fetch-Site')=='cross-site':return self.respond(403,{'error':'Host/site rejected'})
  url=urllib.parse.urlsplit(self.path)
  if url.path=='/health':return self.respond(200,{'service':'schema-update-server','version':1})
  if url.path=='/':
   key,_=self.session(True)
   if not key:return self.respond(429,{'error':'Session limit reached'})
   return self.respond(200,(pathlib.Path(__file__).parent/'ui/index.html').read_bytes(),'text/html; charset=utf-8',key)
  _,csrf=self.session()
  if not csrf:return self.respond(401,{'error':'Open the local dashboard first'})
  if url.path=='/api/state':return self.respond(200,{**self.server.manager.state(),'csrf':csrf})
  if url.path=='/api/events':return self.respond(200,{'events':self.server.manager.store.db.events(urllib.parse.parse_qs(url.query).get('job',[None])[0])})
  files={'/app.js':('app.js','text/javascript'),'/style.css':('style.css','text/css')}
  if url.path in files:
   name,mime=files[url.path];return self.respond(200,(pathlib.Path(__file__).parent/'ui'/name).read_bytes(),mime+'; charset=utf-8')
  return self.respond(404,{'error':'Not found'})
 def do_POST(self):
  if not self.host()or self.headers.get('Origin')!='http://'+self.headers.get('Host',''):return self.respond(403,{'error':'Origin rejected'})
  _,csrf=self.session();provided=self.headers.get('X-Update-CSRF','')
  if not csrf or not secrets.compare_digest(csrf,provided):return self.respond(403,{'error':'CSRF/session rejected'})
  if self.path!='/api/action':return self.respond(404,{'error':'Not found'})
  if self.headers.get('Transfer-Encoding')or not self.headers.get('Content-Type','').startswith('application/json'):return self.respond(400,{'error':'JSON framing required'})
  try:
   length=int(self.headers.get('Content-Length','0'))
   if not 0<length<=4096:raise ValueError('Invalid request size')
   value=json.loads(self.rfile.read(length))
   if not isinstance(value,dict):raise ValueError('Expected an action object')
   self.respond(202,self.server.manager.action(value))
  except(ValueError,TypeError,KeyError)as error:self.respond(400,{'error':str(error)})
