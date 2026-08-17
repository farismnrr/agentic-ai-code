#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, threading, time, urllib.error, urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
RELAY,ROOT=sys.argv[1:]; P='2026-07-28'; O='http://localhost:3333'
def free_port():
 s=socket.socket(); s.bind(('127.0.0.1',0)); p=s.getsockname()[1]; s.close(); return p
class MockHandler(BaseHTTPRequestHandler):
 def do_GET(self):
  if self.path.startswith('/search?'):
   payload=json.dumps({'results':[{'title':'Mock result','url':'https://example.test/result','content':'workspace regression'}]}).encode()
  elif self.path=='/fetch': payload=b'http-fetch-ok'
  else: self.send_response(404); self.end_headers(); return
  self.send_response(200); self.send_header('Content-Type','application/json' if self.path.startswith('/search?') else 'text/plain'); self.send_header('Content-Length',str(len(payload))); self.end_headers(); self.wfile.write(payload)
 def log_message(self,*_): pass
def wait(port,proc):
 for _ in range(100):
  if proc.poll() is not None: raise AssertionError(proc.stderr.read())
  try:
   with urllib.request.urlopen(f'http://127.0.0.1:{port}/health',timeout=1) as r:
    if r.status==200:return
  except Exception: time.sleep(.05)
 raise AssertionError('relay health timeout')
def mcp(method,params=None,i=1):
 params=dict(params or {}); params.setdefault('_meta',{'io.modelcontextprotocol/protocolVersion':P,'io.modelcontextprotocol/clientCapabilities':{}}); return {'jsonrpc':'2.0','id':i,'method':method,'params':params}
def req(url,method,name=None,args=None,i=1):
 h={'Content-Type':'application/json','Origin':O,'MCP-Protocol-Version':P,'Mcp-Method':method};
 if name:h['Mcp-Name']=name
 params={} if method=='tools/list' else {'name':name,'arguments':args or {}}
 r=urllib.request.Request(url,data=json.dumps(mcp(method,params,i)).encode(),headers=h,method='POST')
 try:
  with urllib.request.urlopen(r,timeout=10) as x: raw=x.read(); return x.status,json.loads(raw)
 except urllib.error.HTTPError as e:
  raw=e.read();
  try:return e.code,json.loads(raw)
  except Exception:return e.code,{'_raw':raw.decode('utf-8','replace')}
def call(url,name,args,i=1):
 st,b=req(url,'tools/call',name,args,i); assert st==200,(name,st,b); r=b['result']; assert r['resultType']=='complete'; return r
def payload(result):
 assert result['isError'] is False,result
 text=next(x['text'] for x in result['content'] if x.get('type')=='text'); return json.loads(text)
def expect_error(url,name,args,label):
 st,b=req(url,'tools/call',name,args,90)
 assert st in (200,400,413),(label,st,b)
 if st==200: assert b['result']['isError'] is True,(label,b)
 elif st==400: assert b['error']['code']==-32602,(label,b)
with tempfile.TemporaryDirectory(prefix='relay-workspace-v1-') as base:
 ws=os.path.join(base,'ws'); ext=os.path.join(base,'ext'); os.makedirs(os.path.join(ws,'src')); os.makedirs(ext)
 open(os.path.join(ws,'src','a.rs'),'w').write('needle one\nneedle two\n')
 open(os.path.join(ws,'src','b.rs'),'w').write('needle three\n')
 target=os.path.join(ws,'edit.txt'); open(target,'w').write('alpha beta\n')
 canary=os.path.join(ext,'CANARY-EXTERNAL-038.txt'); open(canary,'w').write('external')
 os.symlink(canary,os.path.join(ws,'external-file-link'))
 os.symlink(ext,os.path.join(ws,'external-dir-link'))
 os.symlink('loop-b',os.path.join(ws,'loop-a')); os.symlink('loop-a',os.path.join(ws,'loop-b'))
 relay=None
 try:
  port=free_port(); relay=subprocess.Popen([RELAY,'relay','--port',str(port),'--dir',ws,'--execution-root',ws,'--origin',O,'--mode','local'],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True); wait(port,relay); url=f'http://127.0.0.1:{port}/mcp'
  st,b=req(url,'tools/list'); assert st==200
  tools={t['name']:t for t in b['result']['tools']}; names=['directory_list','file_search','text_search','file_read','file_edit','file_write']
  for name in names:
   assert name in tools,name; assert tools[name]['inputSchema']['additionalProperties'] is False; assert tools[name]['securitySchemes']==[{'type':'oauth2','scopes':['relay.coding']}]
  x=payload(call(url,'directory_list',{'path':'.','depth':2,'max_entries':1})); assert x['truncated'] is True and len(x['entries'])==1
  x=payload(call(url,'file_search',{'pattern':'**/*.rs','max_results':1})); assert x['truncated'] is True and x['count']==1
  x=payload(call(url,'text_search',{'query':'needle','max_results':1})); assert x['truncated'] is True and x['count']==1
  x=payload(call(url,'file_read',{'path':'src/a.rs','limit_lines':1})); assert x['truncated'] is True and x['start_line']==1
  x=payload(call(url,'file_edit',{'path':'edit.txt','old_text':'alpha','new_text':'ALPHA'})); assert x['replacements']==1 and open(target).read()=='ALPHA beta\n'
  x=payload(call(url,'file_write',{'path':'created.txt','content':'created'})); assert x['created'] is True and open(os.path.join(ws,'created.txt')).read()=='created'
  # Schema matrix: missing required, unknown, wrong type, oversized, invalid range. Mutating cases prove pre-dispatch by unchanged sentinel/absence.
  expect_error(url,'text_search',{},'missing required query')
  marker=os.path.join(ws,'schema-marker.txt'); expect_error(url,'file_write',{'path':'schema-marker.txt','content':'x','extra':1},'unknown property'); assert not os.path.exists(marker)
  before=open(target).read(); expect_error(url,'file_edit',{'path':'edit.txt','old_text':'ALPHA','new_text':'x','replace_all':'yes'},'wrong type'); assert open(target).read()==before
  expect_error(url,'file_read',{'path':'x'*65537},'oversized path')
  expect_error(url,'directory_list',{'depth':5},'invalid range')
  # Filesystem security matrix through MCP.
  for tool,args,label in [
   ('file_read',{'path':'../escape'},'parent traversal read'),('file_read',{'path':'../../etc/passwd'},'nested traversal read'),('file_read',{'path':canary},'absolute external read'),('file_read',{'path':'external-file-link'},'external file symlink'),('directory_list',{'path':'external-dir-link'},'external directory symlink'),('directory_list',{'path':'loop-a'},'recursive symlink loop'),('file_write',{'path':'external-dir-link/pwn.txt','content':'pwn','create_parents':True},'write external symlink parent'),('file_write',{'path':'external-dir-link/new/pwn.txt','content':'pwn','create_parents':True},'new path external symlink ancestor')]:
   expect_error(url,tool,args,label)
  assert open(canary).read()=='external' and not os.path.exists(os.path.join(ext,'pwn.txt'))
  # Existing tool regressions. Direct argv values beginning with '-' or '--'
  # must remain ordinary child-process arguments for both sync and job paths.
  r=call(url,'terminal_exec',{'command':'printf','args':['%s %s','--help','--locked']}); terminal_text=json.dumps(r['content']); assert r['isError'] is False and '--help --locked' in terminal_text,terminal_text
  flag_job=payload(call(url,'terminal_job_start',{'command':'printf','args':['%s','--job-flag']})); flag_jid=flag_job['taskId']
  for _ in range(100):
   flag_snap=payload(call(url,'terminal_job_get',{'taskId':flag_jid}));
   if flag_snap['status']=='completed': break
   time.sleep(.02)
  assert flag_snap['status']=='completed' and flag_snap['output']['stdout']=='--job-flag',flag_snap
  started=payload(call(url,'terminal_job_start',{'command':'sh','args':['-c','sleep 3']})); jid=started['taskId']; snap=payload(call(url,'terminal_job_get',{'taskId':jid})); assert snap['status'] in ('queued','working'); payload(call(url,'terminal_job_cancel',{'taskId':jid}))
  r=call(url,'http_fetch',{'url':'http://127.0.0.1:8888/search'}); http_text=json.dumps(r['content']); assert r['isError'] is True and 'SSRF guard blocked request to private/local IP' in http_text,http_text
  r=call(url,'web_search',{'query':'example domain'}); web_text=json.dumps(r['content']); assert r['isError'] is False and 'Error:' not in web_text and 'Search failed' not in web_text
  r=call(url,'terminal_exec',{'command':'docker','args':['version']}); assert r['isError'] is True and 'RELAY_ALLOW_DOCKER=true' in json.dumps(r['content'])
  print('workspace v1 integration acceptance: PASS')
 finally:
  if relay is not None:
   relay.terminate();
   try: relay.wait(timeout=5)
   except subprocess.TimeoutExpired: relay.kill()

PY
