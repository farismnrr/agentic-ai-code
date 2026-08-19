#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request
RELAY, ROOT = sys.argv[1:]
P='2026-07-28'; O='http://localhost:3333'
def free_port():
 s=socket.socket(); s.bind(('127.0.0.1',0)); p=s.getsockname()[1]; s.close(); return p
def wait(port,proc):
 for _ in range(100):
  if proc.poll() is not None: raise AssertionError(proc.stderr.read())
  try:
   with urllib.request.urlopen(f'http://127.0.0.1:{port}/health',timeout=.5) as r:
    if r.status==200:return
  except Exception: time.sleep(.05)
 raise AssertionError('relay health timeout')
def rpc(url,method,params,i,name=None):
 params=dict(params); params['_meta']={'io.modelcontextprotocol/protocolVersion':P,'io.modelcontextprotocol/clientCapabilities':{}}
 h={'Accept':'application/json','Content-Type':'application/json','Origin':O,'MCP-Protocol-Version':P,'Mcp-Method':method}
 if name:h['Mcp-Name']=name
 req=urllib.request.Request(url,data=json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params}).encode(),headers=h,method='POST')
 try:
  with urllib.request.urlopen(req,timeout=5) as r:return r.status,json.loads(r.read())
 except urllib.error.HTTPError as e:
  raw=e.read().decode('utf-8','replace')
  try:return e.code,json.loads(raw)
  except Exception:return e.code,{'_raw':raw}
def call(url,name,args,i):
 st,b=rpc(url,'tools/call',{'name':name,'arguments':args},i,name); assert st==200,(name,st,b); return b['result']
def payload(r):
 assert r['resultType']=='complete' and r.get('isError') is False,r
 return json.loads(next(x['text'] for x in r['content'] if x.get('type')=='text'))
def expect_error(url,name,args,i):
 st,b=rpc(url,'tools/call',{'name':name,'arguments':args},i,name); assert st in (200,400),(name,st,b)
 if st==200: assert b['result']['isError'] is True,b
with tempfile.TemporaryDirectory(prefix='relay-040c-') as owner:
 repo=os.path.join(owner,'repo'); os.makedirs(repo)
 subprocess.run(['git','-C',repo,'init','-q','-b','main'],check=True)
 open(os.path.join(repo,'a.txt'),'w').write('a\n')
 subprocess.run(['git','-C',repo,'add','a.txt'],check=True)
 subprocess.run(['git','-C',repo,'-c','user.name=fixture','-c','user.email=fixture@example.test','commit','-qm','base'],check=True)
 subprocess.run(['git','-C',repo,'remote','add','origin','git@github.com:farismnrr/ai-code.git'],check=True)
 port=free_port(); proc=subprocess.Popen([RELAY,'relay','--port',str(port),'--dir',repo,'--execution-root',owner,'--origin',O,'--mode','local'],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  wait(port,proc); url=f'http://127.0.0.1:{port}/mcp'
  st,listed=rpc(url,'tools/list',{},1); assert st==200,listed
  tools={x['name']:x for x in listed['result']['tools']}
  for name in ['git_remote_list','git_remote_branch_get','git_fetch','git_push','git_remote_branch_delete']:assert name in tools,name
  assert tools['git_remote_list']['annotations']['readOnlyHint'] is True
  assert tools['git_push']['annotations']['openWorldHint'] is True and tools['git_push']['annotations']['destructiveHint'] is True
  result=payload(call(url,'git_remote_list',{'cwd':'repo'},10))
  assert result['remotes']==[{'name':'origin','provider':'github','owner':'farismnrr','repository':'ai-code','canonical_url':'https://github.com/farismnrr/ai-code.git'}],result
  subprocess.run(['git','-C',repo,'remote','set-url','origin','https://example.com/x/y.git'],check=True)
  expect_error(url,'git_remote_list',{'cwd':'repo'},20)
  subprocess.run(['git','-C',repo,'remote','set-url','origin','https://github.com/farismnrr/ai-code.git'],check=True)
  subprocess.run(['git','-C',repo,'config','credential.helper','evil-helper'],check=True)
  expect_error(url,'git_remote_list',{'cwd':'repo'},30)
  subprocess.run(['git','-C',repo,'config','--unset-all','credential.helper'],check=True)
  expect_error(url,'git_push',{'cwd':'repo','branch':'refs/heads/bad'},40)
  expect_error(url,'git_remote_branch_delete',{'cwd':'repo','branch':'main','expected_sha':'bad'},50)
  print('040C remote Git contract acceptance: PASS')
 finally:
  proc.terminate()
  try:proc.wait(timeout=3)
  except subprocess.TimeoutExpired:proc.kill();proc.wait(timeout=3)
PY
