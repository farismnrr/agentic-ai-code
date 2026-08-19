#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request
RELAY,ROOT=sys.argv[1:]; P='2026-07-28'; O='http://localhost:3333'
def port():
 s=socket.socket();s.bind(('127.0.0.1',0));p=s.getsockname()[1];s.close();return p
def wait(p,proc):
 for _ in range(100):
  if proc.poll() is not None:raise AssertionError(proc.stderr.read())
  try:
   with urllib.request.urlopen(f'http://127.0.0.1:{p}/health',timeout=.5) as r:
    if r.status==200:return
  except Exception:time.sleep(.05)
 raise AssertionError('health timeout')
def rpc(url,method,params,i,name=None):
 params=dict(params);params['_meta']={'io.modelcontextprotocol/protocolVersion':P,'io.modelcontextprotocol/clientCapabilities':{}}
 h={'Accept':'application/json','Content-Type':'application/json','Origin':O,'MCP-Protocol-Version':P,'Mcp-Method':method}
 if name:h['Mcp-Name']=name
 q=urllib.request.Request(url,data=json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params}).encode(),headers=h,method='POST')
 try:
  with urllib.request.urlopen(q,timeout=5) as r:return r.status,json.loads(r.read())
 except urllib.error.HTTPError as e:
  raw=e.read().decode('utf-8','replace')
  try:return e.code,json.loads(raw)
  except Exception:return e.code,{'_raw':raw}
def expect_error(url,name,args,i):
 st,b=rpc(url,'tools/call',{'name':name,'arguments':args},i,name);assert st in (200,400),(name,st,b)
 if st==200:assert b['result']['isError'] is True,(name,b)
with tempfile.TemporaryDirectory(prefix='relay-040de-') as owner:
 repo=os.path.join(owner,'repo');os.makedirs(repo)
 subprocess.run(['git','-C',repo,'init','-q','-b','main'],check=True)
 open(os.path.join(repo,'a'),'w').write('a\n');subprocess.run(['git','-C',repo,'add','a'],check=True)
 subprocess.run(['git','-C',repo,'-c','user.name=fixture','-c','user.email=fixture@example.test','commit','-qm','base'],check=True)
 subprocess.run(['git','-C',repo,'remote','add','origin','https://github.com/farismnrr/ai-code.git'],check=True)
 p=port();proc=subprocess.Popen([RELAY,'relay','--port',str(p),'--dir',repo,'--execution-root',owner,'--origin',O,'--mode','local'],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  wait(p,proc);url=f'http://127.0.0.1:{p}/mcp';st,b=rpc(url,'tools/list',{},1);assert st==200,b
  tools={x['name']:x for x in b['result']['tools']}
  expected=['change_request_list','change_request_get','change_request_create','change_request_update','change_request_checks','change_request_merge']
  for name in expected:assert name in tools,name
  for name in ['change_request_list','change_request_get','change_request_checks']:
   assert tools[name]['annotations']['readOnlyHint'] is True and tools[name]['annotations']['openWorldHint'] is True,name
  for name in ['change_request_create','change_request_update','change_request_merge']:
   assert tools[name]['annotations']['readOnlyHint'] is False and tools[name]['annotations']['destructiveHint'] is True,name
  for name in expected:
   schema=tools[name]['inputSchema'];assert schema.get('additionalProperties') is False,name
   props=schema.get('properties',{})
   for forbidden in ['url','repo','repository','owner','command','args','endpoint','api','admin','force','auto']:
    assert forbidden not in props,(name,forbidden)
  expect_error(url,'change_request_list',{'cwd':'repo','state':'evil'},10)
  expect_error(url,'change_request_create',{'cwd':'repo','head_branch':'refs/heads/x','base_branch':'main','title':'x','body':''},20)
  expect_error(url,'change_request_update',{'cwd':'repo','number':1},30)
  expect_error(url,'change_request_merge',{'cwd':'repo','number':1,'expected_head_sha':'bad','strategy':'squash'},40)
  expect_error(url,'change_request_merge',{'cwd':'repo','number':1,'expected_head_sha':'0'*40,'strategy':'admin'},50)
  subprocess.run(['git','-C',repo,'config','credential.helper','evil'],check=True)
  expect_error(url,'change_request_list',{'cwd':'repo'},60)
  print('040D/E forge contract acceptance: PASS')
 finally:
  proc.terminate()
  try:proc.wait(timeout=3)
  except subprocess.TimeoutExpired:proc.kill();proc.wait(timeout=3)
PY
