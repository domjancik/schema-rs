#!/usr/bin/python3
"""Local appliance login and bounded, authenticated proxy to the Schéma engine."""
import argparse
import collections
import hmac
import http.client
import http.cookies
import http.server
import json
import os
import pathlib
import re
import secrets
import socket
import threading
import time
import urllib.parse

MAX_BODY = 2 * 1024 * 1024
HOP_HEADERS = {'connection', 'proxy-connection', 'keep-alive', 'transfer-encoding',
               'upgrade', 'te', 'trailer', 'proxy-authenticate', 'proxy-authorization'}
PROBES = {'/generate_204', '/gen_204', '/hotspot-detect.html', '/connecttest.txt',
          '/ncsi.txt', '/success.txt', '/connectivity-check.html'}
STYLE = """body{margin:0;background:#111b17;color:#dceae2;font:16px/1.5 system-ui;
min-height:100dvh;display:grid;place-items:center}main{box-sizing:border-box;
width:min(430px,92vw);padding:28px;border:1px solid #426355;border-radius:16px;
background:#182820}h1{font-size:28px;margin:8px 0}p{opacity:.8}small{display:block;
margin-top:24px;opacity:.65}input,button{box-sizing:border-box;width:100%;
font:inherit;border-radius:9px;padding:14px;margin:10px 0}input{background:#102019;
color:inherit;border:1px solid #668b78;font-size:24px;letter-spacing:.3em}
button,a.button{background:#72dec2;color:#102019;border:0;cursor:pointer;
font-weight:600}a{color:#72dec2}label{display:block;margin-top:24px}
.brand{font-size:11px;letter-spacing:.2em;color:#72dec2}.error{color:#ffbda7}"""


class AuthState:
    def __init__(self):
        self.lock = threading.RLock()
        self.sessions = {}
        self.challenges = {}
        self.attempts = {}
        self.global_attempts = collections.deque()

    def clean(self):
        now = time.monotonic()
        for table in (self.sessions, self.challenges):
            for key in list(table):
                if table[key][1] <= now:
                    del table[key]
        for key in list(self.attempts):
            recent = self.attempts[key]
            while recent and recent[0] <= now - 60:
                recent.popleft()
            if not recent:
                del self.attempts[key]
        while self.global_attempts and self.global_attempts[0] <= now - 60:
            self.global_attempts.popleft()

    def challenge(self, peer):
        with self.lock:
            self.clean()
            if len(self.challenges) >= 512:
                self.challenges.pop(next(iter(self.challenges)))
            nonce = secrets.token_urlsafe(32)
            self.challenges[nonce] = (peer, time.monotonic() + 600)
            return nonce

    def valid(self, token, peer):
        with self.lock:
            self.clean()
            return token in self.sessions and self.sessions[token][0] == peer

    def login(self, nonce, cookie_nonce, peer, pin, expected):
        with self.lock:
            self.clean()
            if (not re.fullmatch(r'[A-Za-z0-9_-]{43}', nonce)
                    or not re.fullmatch(r'[A-Za-z0-9_-]{43}', cookie_nonce)
                    or not hmac.compare_digest(nonce, cookie_nonce)):
                return 403, None
            challenge = self.challenges.pop(nonce, None)
            if not challenge or challenge[0] != peer:
                return 403, None
            recent = self.attempts.get(peer, collections.deque())
            if len(recent) >= 5 or len(self.global_attempts) >= 32:
                return 429, None
            if peer not in self.attempts and len(self.attempts) >= 256:
                return 429, None
            if not hmac.compare_digest(pin.encode('utf-8'), expected.encode('ascii')):
                recent.append(time.monotonic())
                self.attempts[peer] = recent
                self.global_attempts.append(time.monotonic())
                return 401, None
            self.attempts.pop(peer, None)
            if len(self.sessions) >= 128:
                self.sessions.pop(next(iter(self.sessions)))
            token = secrets.token_urlsafe(32)
            self.sessions[token] = (peer, time.monotonic() + 8 * 3600)
            return 303, token


class PortalServer(http.server.ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, address, config):
        token = config.get('schema_token', '')
        pin = config.get('admin_pin', '')
        if not re.fullmatch(r'[A-Za-z0-9_-]{16,256}', token):
            raise ValueError('Invalid engine token configuration')
        if not re.fullmatch(r'[0-9]{6}', pin):
            raise ValueError('Admin PIN must be six ASCII digits')
        hosts = set(config.get('allowed_hosts', []))
        if not hosts or any(not re.fullmatch(r'[a-z0-9.-]{1,253}', h) for h in hosts):
            raise ValueError('Invalid portal host configuration')
        if config.get('upstream_host', '127.0.0.1') != '127.0.0.1':
            raise ValueError('Engine upstream must be IPv4 loopback')
        self.upstream_port = int(config.get('upstream_port', 8787))
        if not 1 <= self.upstream_port <= 65535:
            raise ValueError('Invalid upstream port')
        self.engine_token, self.pin, self.hosts = token, pin, hosts
        self.auth = AuthState()
        self.workers = threading.BoundedSemaphore(16)
        super().__init__(address, PortalHandler)
        self.canonical = config.get('canonical_origin', 'http://127.0.0.1:' + str(self.server_port))
        origin = urllib.parse.urlsplit(self.canonical)
        if (origin.scheme != 'http' or origin.hostname not in hosts or origin.username
                or origin.password or origin.path or origin.query or origin.fragment):
            self.server_close()
            raise ValueError('Invalid canonical portal origin')

    def process_request(self, request, address):
        if not self.workers.acquire(blocking=False):
            self.shutdown_request(request)
            return
        try:
            super().process_request(request, address)
        except BaseException:
            self.workers.release()
            raise

    def process_request_thread(self, request, address):
        try:
            super().process_request_thread(request, address)
        finally:
            self.workers.release()


class PortalHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.0'
    server_version = 'SchemaPortal'
    sys_version = ''

    def setup(self):
        super().setup()
        self.connection.settimeout(10)

    def log_message(self, *args):
        # Never log login bodies, headers, cookies or engine credentials.
        pass

    def respond(self, status, body=b'', content_type='text/html; charset=utf-8', headers=None):
        if isinstance(body, str):
            body = body.encode('utf-8')
        self.send_response(status)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.send_header('Content-Security-Policy', "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'")
        for key, value in (headers or {}).items():
            self.send_header(key, value)
        self.end_headers()
        if self.command != 'HEAD':
            self.wfile.write(body)

    def redirect(self, target, status=302, headers=None):
        self.respond(status, headers={'Location': target, **(headers or {})})

    def cookie(self, key):
        try:
            cookie = http.cookies.SimpleCookie(self.headers.get('Cookie', ''))
            return cookie[key].value if key in cookie else ''
        except http.cookies.CookieError:
            return ''

    def host(self):
        values = self.headers.get_all('Host', [])
        if len(values) != 1:
            return None
        host = values[0].lower()
        parts = host.rsplit(':', 1)
        if len(parts) == 2:
            if parts[1] != str(self.server.server_port):
                return None
            host = parts[0]
        return values[0].lower() if host.rstrip('.') in self.server.hosts else None

    def framing(self):
        if (self.headers.get('Transfer-Encoding') is not None
                or len(self.headers.get_all('Content-Length', [])) > 1
                or sum(len(k) + len(v) for k, v in self.headers.items()) > 16384):
            self.respond(400, 'Unsupported request framing')
            return None
        try:
            length = int(self.headers.get('Content-Length', '0'))
        except ValueError:
            length = -1
        if length < 0:
            self.respond(400, 'Invalid content length')
            return None
        if length > MAX_BODY:
            self.respond(413, 'Request is too large')
            return None
        return length

    def login_page(self, status=200, error=''):
        nonce = self.server.auth.challenge(self.client_address[0])
        html = ('<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1">'
                '<title>Schéma device login</title><style>' + STYLE + '</style></head><body><main>'
                '<div class="brand">SCHEMA / LOCAL CONTROL</div><h1>Your room. Your controls.</h1>'
                '<p>Sign in to open the lighting control panel.</p><p class="error">' + error + '</p>'
                '<form method="post" action="/login"><input type="hidden" name="nonce" value="' + nonce + '">'
                '<label for="pin">Device PIN</label><input id="pin" name="pin" type="password" inputmode="numeric" '
                'pattern="[0-9]{6}" maxlength="6" autocomplete="current-password" required autofocus>'
                '<button type="submit">Open controls</button></form><small>This Wi-Fi network provides local '
                'lighting control. Internet access is not provided.</small></main></body></html>')
        headers = {'Set-Cookie': 'schema_login=' + nonce + '; Path=/login; Max-Age=600; HttpOnly; SameSite=Strict'}
        if status == 429:
            headers['Retry-After'] = '60'
        self.respond(status, html, headers=headers)

    def authenticated(self):
        return self.server.auth.valid(self.cookie('schema_admin'), self.client_address[0])

    def do_HEAD(self):
        self.do_GET()

    def do_GET(self):
        length = self.framing()
        if length is None:
            return
        if not self.path.startswith('/') or self.path.startswith('//'):
            return self.respond(400, 'Origin-form requests required')
        host = self.host()
        path = urllib.parse.urlsplit(self.path).path
        if not host:
            if path.startswith('/api/'):
                return self.respond(403, 'Host not permitted')
            return self.redirect(self.server.canonical + '/login')
        if path == '/portal/health':
            return self.respond(200, '{"ok":true}', 'application/json')
        if path == '/login':
            return self.login_page()
        if not self.authenticated():
            if path.startswith('/api/'):
                return self.respond(401, '{"error":"Device login required. Reload this page to sign in."}', 'application/json')
            return self.redirect('/login')
        if path == '/admin' or path in PROBES:
            return self.redirect('/')
        self.proxy()

    def do_POST(self):
        length = self.framing()
        if length is None:
            return
        if not self.path.startswith('/') or self.path.startswith('//'):
            return self.respond(400, 'Origin-form requests required')
        host = self.host()
        if not host or self.headers.get('Origin') != 'http://' + host:
            return self.respond(403, 'Origin not permitted')
        body = self.rfile.read(length)
        if len(body) != length:
            return self.respond(400, 'Incomplete body')
        path = urllib.parse.urlsplit(self.path).path
        if path == '/login':
            if length > 4096 or self.headers.get_content_type() != 'application/x-www-form-urlencoded':
                return self.respond(400, 'Invalid login request')
            try:
                data = urllib.parse.parse_qs(body.decode('utf-8'), max_num_fields=4, strict_parsing=True)
                if set(data) != {'nonce', 'pin'} or any(len(v) != 1 for v in data.values()):
                    raise ValueError('Duplicate or missing login fields')
                nonce, pin = data['nonce'][0], data['pin'][0]
            except (ValueError, UnicodeError):
                return self.respond(400, 'Invalid login form')
            status, token = self.server.auth.login(nonce, self.cookie('schema_login'), self.client_address[0], pin, self.server.pin)
            if status == 303:
                return self.redirect('/admin', 303, {'Set-Cookie': 'schema_admin=' + token + '; Path=/; Max-Age=28800; HttpOnly; SameSite=Strict'})
            if status == 403:
                return self.respond(403, 'This login expired. Open /login again.')
            return self.login_page(status, 'Try again in a minute.' if status == 429 else 'PIN not recognized. Try again.')
        if not self.authenticated():
            return self.respond(401, '{"error":"Device login required"}', 'application/json')
        if path == '/portal/logout':
            with self.server.auth.lock:
                self.server.auth.sessions.pop(self.cookie('schema_admin'), None)
            return self.redirect('/login', 303, {'Set-Cookie': 'schema_admin=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict'})
        self.proxy(body)

    def proxy(self, body=None):
        connection = http.client.HTTPConnection('127.0.0.1', self.server.upstream_port, timeout=10)
        forbidden = HOP_HEADERS | {'host', 'origin', 'cookie', 'content-length', 'x-schema-token'}
        forbidden.update(h.strip().lower() for h in self.headers.get('Connection', '').split(','))
        headers = {k: v for k, v in self.headers.items() if k.lower() not in forbidden}
        headers['Host'] = '127.0.0.1:' + str(self.server.upstream_port)
        headers['x-schema-token'] = self.server.engine_token
        if self.headers.get('Origin'):
            headers['Origin'] = 'http://' + headers['Host']
        started = False
        try:
            connection.request(self.command, self.path, body=body, headers=headers)
            response = connection.getresponse()
            if response.getheader('Content-Length') and int(response.getheader('Content-Length')) > 64 * 1024 * 1024:
                return self.respond(502, 'Engine response is too large')
            started = True
            self.send_response(response.status)
            excluded = HOP_HEADERS | {'server', 'date', 'set-cookie', 'cache-control'}
            excluded.update(h.strip().lower() for h in response.getheader('Connection', '').split(','))
            for key, value in response.getheaders():
                if key.lower() not in excluded:
                    self.send_header(key, value)
            self.send_header('Cache-Control', 'no-store')
            self.end_headers()
            if self.command != 'HEAD':
                remaining = 64 * 1024 * 1024
                while remaining > 0:
                    data = response.read(min(65536, remaining))
                    if not data:
                        break
                    self.wfile.write(data)
                    remaining -= len(data)
        except (OSError, http.client.HTTPException, ValueError):
            self.close_connection = True
            if not started:
                self.respond(502, 'Lighting engine is temporarily unavailable.')
        finally:
            connection.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', default=str(pathlib.Path(os.environ.get('CREDENTIALS_DIRECTORY', '/etc/schema')) / 'portal.json'))
    parser.add_argument('--bind', default='0.0.0.0')
    parser.add_argument('--port', type=int, default=80)
    args = parser.parse_args()
    config = json.loads(pathlib.Path(args.config).read_text())
    server = PortalServer((args.bind, args.port), config)
    try:
        server.serve_forever()
    finally:
        server.server_close()


if __name__ == '__main__':
    main()
