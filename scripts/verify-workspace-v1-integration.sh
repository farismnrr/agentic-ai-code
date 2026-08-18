#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, shutil, socket, subprocess, sys, tempfile, threading, time, urllib.error, urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
RELAY,ROOT=sys.argv[1:]; P='2026-07-28'; O='http://localhost:3333'; ALLOW_TERMINAL_NETWORK=os.environ.get('ALLOW_TERMINAL_NETWORK')=='1'
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
 st,b=req(url,'tools/call',name,args,i); time.sleep(.14); assert st==200,(name,st,b); r=b['result']; assert r['resultType']=='complete'; return r
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
 os.makedirs(os.path.join(ws,'.ssh')); open(os.path.join(ws,'.ssh','id_test'),'w').write('protected-canary')
 open(os.path.join(ws,'.npmrc'),'w').write('protected-canary')
 open(os.path.join(ws,'.git-credentials'),'w').write('protected-git-credentials-canary')
 os.makedirs(os.path.join(ws,'.config','gh'))
 open(os.path.join(ws,'.config','gh','hosts.yml'),'w').write('protected-gh-canary')
 open(os.path.join(ws,'.env'),'w').write('protected-env-canary')
 open(os.path.join(ws,'.env.local'),'w').write('protected-env-local-canary')
 os.makedirs(os.path.join(ws,'nested'))
 open(os.path.join(ws,'nested','.env'),'w').write('protected-nested-env-canary')
 open(os.path.join(ws,'nested','.env.production'),'w').write('protected-nested-env-production-canary')
 open(os.path.join(ws,'.ssh-cache'),'w').write('near-miss')
 open(os.path.join(ws,'.npmrc.bak'),'w').write('near-miss')
 open(os.path.join(ws,'.env.example'),'w').write('EXAMPLE=ok')
 open(os.path.join(ws,'nested','.env.example'),'w').write('NESTED_EXAMPLE=ok')
 os.symlink('.ssh/id_test',os.path.join(ws,'innocent.txt'))
 open(os.path.join(ws,'src','a.rs'),'w').write('needle one\nneedle two\n')
 open(os.path.join(ws,'src','b.rs'),'w').write('needle three\n')
 target=os.path.join(ws,'edit.txt'); open(target,'w').write('alpha beta\n')
 canary=os.path.join(ext,'CANARY-EXTERNAL-038.txt'); open(canary,'w').write('external')
 os.symlink(canary,os.path.join(ws,'external-file-link'))
 os.symlink(ext,os.path.join(ws,'external-dir-link'))
 os.symlink('loop-b',os.path.join(ws,'loop-a')); os.symlink('loop-a',os.path.join(ws,'loop-b'))
 # Synthetic operator-approved toolchains prove Bubblewrap can expose the
 # minimum read-only runtime state without rebinding the owner's whole home.
 toolhome=os.path.join(ws,'.synthetic-toolhome'); cargo_bin=os.path.join(toolhome,'.cargo','bin'); rustup_home=os.path.join(toolhome,'.rustup')
 os.makedirs(cargo_bin); os.makedirs(rustup_home)
 open(os.path.join(toolhome,'.cargo','credentials'),'w').write('synthetic-cargo-secret')
 open(os.path.join(rustup_home,'marker'),'w').write('synthetic-rustup-ok')
 rustup=os.path.join(cargo_bin,'rustup')
 open(rustup,'w').write(f'''#!/bin/sh\n[ "${{RUSTUP_HOME:-}}" = "{rustup_home}" ] || exit 41\n[ -r "{rustup_home}/marker" ] || exit 42\n[ ! -s "{os.path.join(toolhome,'.cargo','credentials')}" ] || exit 43\nprintf 'synthetic-cargo-ok\\n'\n''')
 os.chmod(rustup,0o755); os.symlink('rustup',os.path.join(cargo_bin,'cargo'))
 node_root=os.path.join(ws,'.synthetic-node-v1'); node_bin=os.path.join(node_root,'bin'); os.makedirs(node_bin); os.makedirs(os.path.join(node_root,'lib','node_modules'))
 node=os.path.join(node_bin,'node'); open(node,'w').write("#!/bin/sh\nprintf 'synthetic-node-ok\\n'\n"); os.chmod(node,0o755)
 fnm_alias=os.path.join(ws,'.synthetic-fnm','aliases'); os.makedirs(fnm_alias); os.symlink(node_root,os.path.join(fnm_alias,'default'))
 relay=None
 mock=ThreadingHTTPServer(('127.0.0.1',0),MockHandler); threading.Thread(target=mock.serve_forever,daemon=True).start()
 try:
  port=free_port(); relay_args=[RELAY,'relay','--port',str(port),'--dir',ws,'--execution-root',ws,'--origin',O,'--mode','local','--toolchain-path',cargo_bin,'--toolchain-path',os.path.join(fnm_alias,'default','bin')]
  if ALLOW_TERMINAL_NETWORK: relay_args.append('--allow-terminal-network')
  relay=subprocess.Popen(relay_args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True); wait(port,relay); url=f'http://127.0.0.1:{port}/mcp'
  st,b=req(url,'tools/list'); assert st==200
  tools={t['name']:t for t in b['result']['tools']}; names=['directory_list','file_search','text_search','file_read','file_edit','file_write','git_show']
  for name in names:
   assert name in tools,name; assert tools[name]['inputSchema']['additionalProperties'] is False; assert tools[name]['securitySchemes']==[{'type':'oauth2','scopes':['relay.coding']}]
  x=payload(call(url,'directory_list',{'path':'.','depth':2,'max_entries':1})); assert x['truncated'] is True and len(x['entries'])==1
  directory_next=x.get('continuation'); assert directory_next; x2=payload(call(url,'directory_list',{'path':'.','depth':2,'max_entries':1,'continuation':directory_next})); assert x2['entries'] and x2['entries'][0]['path'] != x['entries'][0]['path']
  x=payload(call(url,'file_search',{'pattern':'**/*.rs','max_results':1})); assert x['truncated'] is True and x['count']==1
  file_next=x.get('continuation'); assert file_next; x2=payload(call(url,'file_search',{'pattern':'**/*.rs','max_results':1,'continuation':file_next})); assert x2['matches'] and x2['matches'][0] != x['matches'][0]
  x=payload(call(url,'text_search',{'query':'needle','max_results':1})); assert x['truncated'] is True and x['count']==1
  text_next=x.get('continuation'); assert text_next; x2=payload(call(url,'text_search',{'query':'needle','max_results':1,'continuation':text_next})); assert x2['matches'] and x2['matches'][0]['path'] == x['matches'][0]['path']
  expect_error(url,'text_search',{'query':'needle','max_results':2,'continuation':text_next},'tampered continuation limit')
  x=payload(call(url,'file_read',{'path':'src/a.rs','limit_lines':1})); assert x['truncated'] is True and x['start_line']==1
  x=payload(call(url,'file_edit',{'path':'edit.txt','old_text':'alpha','new_text':'ALPHA'})); assert x['replacements']==1 and open(target).read()=='ALPHA beta\n'
  x=payload(call(url,'file_write',{'path':'created.txt','content':'created'})); assert x['created'] is True and open(os.path.join(ws,'created.txt')).read()=='created'
  # Protected-path acceptance: direct reads, contained symlink aliases, and
  # recursive discovery must fail closed or omit both names and content.
  for tool,args,label in [
   ('file_read',{'path':'.ssh/id_test'},'protected direct read'),
   ('file_read',{'path':'.env'},'protected env read'),
   ('file_read',{'path':'.git-credentials'},'protected git credentials read'),
   ('file_read',{'path':'.config/gh/hosts.yml'},'protected GitHub CLI credentials read'),
   ('file_read',{'path':'.env.local'},'protected env variant read'),
   ('file_read',{'path':'nested/.env'},'protected nested env read'),
   ('file_read',{'path':'nested/.env.production'},'protected nested env variant read'),
   ('file_read',{'path':'innocent.txt'},'protected symlink alias'),
   ('directory_list',{'path':'.','depth':2},'protected recursive listing'),
   ('file_search',{'pattern':'**/*'},'protected recursive search'),
   ('text_search',{'query':'protected-canary'},'protected text search')]:
   if tool in ('directory_list','file_search','text_search'):
    result=payload(call(url,tool,args))
    rendered=json.dumps(result)
    assert all(secret not in rendered for secret in ['id_test','protected-canary','protected-git-credentials-canary','protected-gh-canary','.git-credentials','.config/gh/hosts.yml']),(label,rendered)
   else:
    expect_error(url,tool,args,label)
  listed=payload(call(url,'directory_list',{'path':'.','depth':2}))['entries']
  assert any(item['path']=='.ssh-cache' for item in listed) and any(item['path']=='.npmrc.bak' for item in listed) and any(item['path']=='.env.example' for item in listed),listed
  # The same protected-path policy must hold at the actual Bubblewrap subprocess
  # boundary, not only in native MCP path validation. Protected files are
  # mounted empty while precise near-miss/example files remain readable.
  expect_error(url,'terminal_exec',{'command':'sh','args':['-c','cat id_test'],'cwd':'.ssh'},'terminal protected cwd rebase')
  expect_error(url,'terminal_exec',{'command':'sh','args':['-c','cat hosts.yml'],'cwd':'.config/gh'},'terminal GitHub credential cwd rebase')
  terminal_env=call(url,'terminal_exec',{'command':'sh','args':['-c','for p in .env .env.local nested/.env nested/.env.production .git-credentials .config/gh/hosts.yml; do cat "$p" 2>/dev/null || true; done; cat .env.example nested/.env.example']})
  terminal_env_text=json.dumps(terminal_env['content'])
  assert terminal_env['isError'] is False,terminal_env
  for protected_canary in ['protected-env-canary','protected-env-local-canary','protected-nested-env-canary','protected-nested-env-production-canary','protected-git-credentials-canary','protected-gh-canary']:
   assert protected_canary not in terminal_env_text,(protected_canary,terminal_env_text)
  assert 'EXAMPLE=ok' in terminal_env_text and 'NESTED_EXAMPLE=ok' in terminal_env_text,terminal_env_text
  # A credential-named symlink is rejected before Bubblewrap setup rather than
  # being followed as a mount destination. Remove it after the adversarial check
  # so subsequent terminal cases exercise their own behavior.
  protected_alias=os.path.join(ws,'nested','.env.symlink')
  os.symlink(canary,protected_alias)
  expect_error(url,'terminal_exec',{'command':'true'},'terminal protected env-variant symlink mount destination')
  os.unlink(protected_alias)
  symlink_case=os.path.join(ws,'symlink-case'); os.makedirs(symlink_case)
  os.symlink(canary,os.path.join(symlink_case,'.env'))
  expect_error(url,'terminal_exec',{'command':'true'},'terminal protected exact env symlink mount destination')
  os.unlink(os.path.join(symlink_case,'.env'))
  os.symlink(ext,os.path.join(symlink_case,'.ssh'))
  expect_error(url,'terminal_exec',{'command':'true'},'terminal protected directory symlink mount destination')
  os.unlink(os.path.join(symlink_case,'.ssh')); os.rmdir(symlink_case)
  # Schema matrix: missing required, unknown, wrong type, oversized, invalid range. Mutating cases prove pre-dispatch by unchanged sentinel/absence.
  expect_error(url,'text_search',{},'missing required query')
  marker=os.path.join(ws,'schema-marker.txt'); expect_error(url,'file_write',{'path':'schema-marker.txt','content':'x','extra':1},'unknown property'); assert not os.path.exists(marker),marker
  before=open(target).read(); expect_error(url,'file_edit',{'path':'edit.txt','old_text':'ALPHA','new_text':'x','replace_all':'yes'},'wrong type'); assert open(target).read()==before
  expect_error(url,'file_read',{'path':'x'*65537},'oversized path')
  expect_error(url,'directory_list',{'depth':5},'invalid range')
  # Filesystem security matrix through MCP.
  for tool,args,label in [
   ('file_read',{'path':'../escape'},'parent traversal read'),('file_read',{'path':'../../etc/passwd'},'nested traversal read'),('file_read',{'path':canary},'absolute external read'),('file_read',{'path':'external-file-link'},'external file symlink'),('directory_list',{'path':'external-dir-link'},'external directory symlink'),('directory_list',{'path':'loop-a'},'recursive symlink loop'),('file_write',{'path':'external-dir-link/pwn.txt','content':'pwn','create_parents':True},'write external symlink parent'),('file_write',{'path':'external-dir-link/new/pwn.txt','content':'pwn','create_parents':True},'new path external symlink ancestor')]:
   expect_error(url,tool,args,label)
  assert open(canary).read()=='external' and not os.path.exists(os.path.join(ext,'pwn.txt'))
  # Operator-approved user toolchains remain usable without exposing the
  # owner's broader home or credential files. Cover both a rustup-style shim
  # and an fnm-style symlinked Node alias.
  r=call(url,'terminal_exec',{'command':'cargo','args':[]}); toolchain_text=json.dumps(r['content']); assert r['isError'] is False and 'synthetic-cargo-ok' in toolchain_text,toolchain_text
  r=call(url,'terminal_exec',{'command':'node','args':[]}); node_text=json.dumps(r['content']); assert r['isError'] is False and 'synthetic-node-ok' in node_text,node_text
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
  terminal_network=call(url,'terminal_exec',{'command':'curl','args':['--max-time','2',f'http://127.0.0.1:{mock.server_port}/fetch']})
  if ALLOW_TERMINAL_NETWORK:
   assert terminal_network['isError'] is False and 'http-fetch-ok' in json.dumps(terminal_network['content'])
  else:
   assert terminal_network['isError'] is True
  subprocess.run(['git','-C',ws,'init','-q'],check=True)
  subprocess.run(['git','-C',ws,'add','.'],check=True)
  subprocess.run(['git','-C',ws,'-c','user.email=fixture@example.test','-c','user.name=fixture','commit','-qm','fixture'],check=True)
  # Create the external-metadata adversarial repo only after the parent fixture
  # commit, otherwise Git correctly refuses to index an unborn nested repo.
  evil_repo=os.path.join(ws,'external-git-metadata')
  evil_git=os.path.join(ext,'git-metadata')
  subprocess.run(['git','init','-q',f'--separate-git-dir={evil_git}',evil_repo],check=True)
  expect_error(url,'git_status',{'cwd':'external-git-metadata'},'external git metadata boundary')
  shutil.rmtree(evil_repo); shutil.rmtree(evil_git)
  # Repo-local metadata must not redirect Git object reads outside its own
  # canonical common directory through object-root symlinks or alternates.
  object_escape=os.path.join(ws,'object-escape'); subprocess.run(['git','init','-q',object_escape],check=True)
  external_objects=os.path.join(ext,'external-objects'); os.makedirs(external_objects)
  shutil.rmtree(os.path.join(object_escape,'.git','objects')); os.symlink(external_objects,os.path.join(object_escape,'.git','objects'))
  expect_error(url,'git_status',{'cwd':'object-escape'},'external git object database symlink')
  shutil.rmtree(object_escape)
  alternates_repo=os.path.join(ws,'alternates-repo'); subprocess.run(['git','init','-q',alternates_repo],check=True)
  alternates_path=os.path.join(alternates_repo,'.git','objects','info','alternates')
  open(alternates_path,'w').write(external_objects+'\n')
  expect_error(url,'git_status',{'cwd':'alternates-repo'},'git alternate object database')
  shutil.rmtree(alternates_repo); shutil.rmtree(external_objects)
  blob=subprocess.check_output(['git','-C',ws,'hash-object','.npmrc'],text=True).strip()
  tree=subprocess.check_output(['git','-C',ws,'rev-parse','HEAD^{tree}'],text=True).strip()
  # Git's revision:path and raw object forms must not reach the presentation
  # command: path exclusions cannot enforce policy after a blob is emitted.
  for ref,label in [
   ('HEAD:.npmrc','protected object path'),
   ('HEAD:.ssh/id_test','protected directory object path'),
   (blob,'raw protected blob object'),
   (tree,'raw tree object'),
   ('HEAD^{tree}','tree-ish expression')]:
   expect_error(url,'git_show',{'ref':ref},label)
  # Tracked protected paths must not leak through any Git surface. The host
  # mutates the fixture directly because native writes to protected paths are
  # intentionally denied.
  open(os.path.join(ws,'.env.local'),'w').write('protected-git-diff-canary\n')
  open(os.path.join(ws,'nested','.env.production'),'w').write('protected-nested-git-diff-canary\n')
  status=payload(call(url,'git_status',{})); status_text=json.dumps(status)
  for protected_name in ['.env.local','nested/.env.production','.git-credentials','.config/gh/hosts.yml']:
   assert protected_name not in status_text,(protected_name,status_text)
  expect_error(url,'git_diff',{'mode':'working'},'git diff containing protected env path')
  normal=payload(call(url,'git_show',{'ref':'HEAD','include_patch':False})); assert 'fixture' in normal['text'],normal
  expect_error(url,'git_show',{'ref':'HEAD'},'git show patch containing protected path')
  example=payload(call(url,'git_show',{'ref':'HEAD','path':'.env.example','include_patch':True})); assert 'EXAMPLE=ok' in example['text'],example
  nested_example=payload(call(url,'git_show',{'ref':'HEAD','path':'nested/.env.example','include_patch':True})); assert 'NESTED_EXAMPLE=ok' in nested_example['text'],nested_example
  near=payload(call(url,'git_show',{'ref':'HEAD','path':'.npmrc.bak','include_patch':False})); assert 'fixture' in near['text'],near
  for tool,args,label in [
   ('git_show',{'ref':'HEAD','path':'.ssh/id_test'},'protected git show path'),
   ('git_show',{'ref':'HEAD','path':'.env.local'},'protected env git show path'),
   ('git_log',{'path':'.env.local'},'protected env git log path'),
   ('git_log',{'path':'nested/.env.production'},'protected nested env git log path'),
   ('git_blame',{'path':'.env.local'},'protected env git blame path')]:
   expect_error(url,tool,args,label)
  log_all=payload(call(url,'git_log',{})); assert 'fixture' in json.dumps(log_all),log_all
  log_example=payload(call(url,'git_log',{'path':'.env.example'})); assert 'fixture' in json.dumps(log_example),log_example
  blame_example=payload(call(url,'git_blame',{'path':'.env.example','start_line':1,'end_line':1})); assert blame_example['lines'],blame_example
  # Staged rename/copy lineage from a protected source must fail closed even
  # when the destination path itself looks safe.
  subprocess.run(['git','-C',ws,'reset','--hard','-q','HEAD'],check=True)
  subprocess.run(['git','-C',ws,'mv','.env.local','safe-renamed.txt'],check=True)
  rename_status=payload(call(url,'git_status',{})); rename_status_text=json.dumps(rename_status)
  assert '.env.local' not in rename_status_text and 'safe-renamed.txt' not in rename_status_text,rename_status
  expect_error(url,'git_diff',{'mode':'staged'},'protected staged rename')
  subprocess.run(['git','-C',ws,'reset','--hard','-q','HEAD'],check=True)
  shutil.copyfile(os.path.join(ws,'.env.local'),os.path.join(ws,'safe-copy.txt'))
  subprocess.run(['git','-C',ws,'add','safe-copy.txt'],check=True)
  copy_status=payload(call(url,'git_status',{})); copy_status_text=json.dumps(copy_status)
  assert 'safe-copy.txt' not in copy_status_text,copy_status
  expect_error(url,'git_diff',{'mode':'staged'},'protected staged copy')
  subprocess.run(['git','-C',ws,'reset','--hard','-q','HEAD'],check=True)
  print('workspace v1 integration acceptance: PASS')
 finally:
  if relay is not None:
   relay.terminate();
   try: relay.wait(timeout=5)
   except subprocess.TimeoutExpired: relay.kill()
  mock.shutdown(); mock.server_close()

PY
