#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null

exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request
RELAY, ROOT = sys.argv[1:]
P='2026-07-28'; O='http://localhost:3333'

def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1',0)); return s.getsockname()[1]

def wait_health(port, proc):
    for _ in range(100):
        if proc.poll() is not None: raise AssertionError(proc.stderr.read())
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=.5) as r:
                if r.status == 200: return
        except Exception: time.sleep(.05)
    raise AssertionError('relay health timeout')

def rpc(url, method, params, i, name=None):
    params=dict(params); params['_meta']={'io.modelcontextprotocol/protocolVersion':P,'io.modelcontextprotocol/clientCapabilities':{}}
    h={'Accept':'application/json','Content-Type':'application/json','Origin':O,'MCP-Protocol-Version':P,'Mcp-Method':method}
    if name: h['Mcp-Name']=name
    req=urllib.request.Request(url,data=json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params}).encode(),headers=h,method='POST')
    try:
        with urllib.request.urlopen(req, timeout=5) as r: return r.status,json.loads(r.read())
    except urllib.error.HTTPError as e:
        raw=e.read().decode('utf-8','replace')
        try: body=json.loads(raw)
        except Exception: body={'_raw':raw}
        return e.code,body

def call(url,name,args,i):
    st,b=rpc(url,'tools/call',{'name':name,'arguments':args},i,name); assert st==200,(name,st,b); return b['result']

def payload(result):
    assert result['resultType']=='complete' and result.get('isError') is False,result
    return json.loads(next(x['text'] for x in result['content'] if x.get('type')=='text'))

def expect_error(url,name,args,label,i):
    st,b=rpc(url,'tools/call',{'name':name,'arguments':args},i,name); assert st in (200,400),(label,st,b)
    if st==200: assert b['result'].get('isError') is True,(label,b)

def host(repo,*args):
    return subprocess.check_output(['git','-C',repo,*args],text=True).strip()

def host_run(repo,*args):
    subprocess.run(['git','-C',repo,*args],check=True,stdout=subprocess.DEVNULL)

with tempfile.TemporaryDirectory(prefix='relay-040b-local-') as owner:
    repo=os.path.join(owner,'repo'); os.makedirs(repo)
    host_run(repo,'init','-q','-b','main')
    host_run(repo,'config','user.name','fixture')
    host_run(repo,'config','user.email','fixture@example.test')
    open(os.path.join(repo,'normal.txt'),'w').write('base\n')
    open(os.path.join(repo,'conflict.txt'),'w').write('base\n')
    host_run(repo,'add','normal.txt','conflict.txt'); host_run(repo,'commit','-qm','base')
    base=host(repo,'rev-parse','HEAD')

    # Prebuild two branch histories with ordinary host Git. Relay operations below
    # exercise only the bounded mutation contract itself.
    host_run(repo,'switch','-q','-c','topic')
    open(os.path.join(repo,'conflict.txt'),'w').write('topic\n'); host_run(repo,'add','conflict.txt'); host_run(repo,'commit','-qm','topic')
    topic=host(repo,'rev-parse','HEAD')
    host_run(repo,'switch','-q','main')
    open(os.path.join(repo,'conflict.txt'),'w').write('main\n'); host_run(repo,'add','conflict.txt'); host_run(repo,'commit','-qm','main-side')
    main_before=host(repo,'rev-parse','HEAD')
    host_run(repo,'branch','rebase-topic',base)

    port=free_port()
    proc=subprocess.Popen([RELAY,'relay','--port',str(port),'--dir',repo,'--execution-root',owner,'--origin',O,'--mode','local'],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:
        wait_health(port,proc); url=f'http://127.0.0.1:{port}/mcp'; cwd={'cwd':'repo'}

        # Explicit stage/unstage/commit.
        open(os.path.join(repo,'normal.txt'),'a').write('relay-change\n')
        staged=payload(call(url,'git_stage',{**cwd,'paths':['normal.txt']},10)); assert staged['paths']==['normal.txt'],staged
        unstaged=payload(call(url,'git_unstage',{**cwd,'paths':['normal.txt']},11)); assert unstaged['paths']==['normal.txt'],unstaged
        payload(call(url,'git_stage',{**cwd,'paths':['normal.txt']},12))
        committed=payload(call(url,'git_commit',{**cwd,'message':'fixture: native commit'},13)); assert committed['branch']=='main' and committed['head']!=main_before,committed
        main_before=committed['head']

        # Protected path may never enter the index through native mutation.
        open(os.path.join(repo,'.env'),'w').write('SECRET=canary\n')
        expect_error(url,'git_stage',{**cwd,'paths':['.env']},'protected stage',20)
        os.unlink(os.path.join(repo,'.env'))

        # Conflicted merge is represented structurally; unresolved continue fails.
        merge=payload(call(url,'git_merge_start',{**cwd,'ref':'topic'},30)); assert merge['operation']=='merge' and merge['conflicts']==['conflict.txt'],merge
        expect_error(url,'git_merge_continue',cwd,'unresolved merge continue',31)
        aborted=payload(call(url,'git_merge_abort',cwd,32)); assert aborted['operation'] is None and aborted['conflicts']==[],aborted
        assert host(repo,'rev-parse','HEAD')==main_before

        # Resolve through ordinary file edit + explicit stage, then continue.
        merge=payload(call(url,'git_merge_start',{**cwd,'ref':'topic'},33)); assert merge['operation']=='merge',merge
        open(os.path.join(repo,'conflict.txt'),'w').write('resolved\n')
        payload(call(url,'git_stage',{**cwd,'paths':['conflict.txt']},34))
        done=payload(call(url,'git_merge_continue',cwd,35)); assert done['operation'] is None and done['conflicts']==[],done
        merged_head=done['head']; assert merged_head!=main_before

        # A clean rebase has bounded completion semantics. Add one commit on a
        # branch rooted at base, then rebase it onto current main.
        payload(call(url,'git_branch_switch',{**cwd,'name':'rebase-topic'},40))
        open(os.path.join(repo,'rebase.txt'),'w').write('rebase\n')
        payload(call(url,'git_stage',{**cwd,'paths':['rebase.txt']},41))
        payload(call(url,'git_commit',{**cwd,'message':'fixture: rebase commit'},42))
        rebased=payload(call(url,'git_rebase_start',{**cwd,'ref':'main'},43)); assert rebased['operation'] is None and rebased['conflicts']==[],rebased
        payload(call(url,'git_branch_switch',{**cwd,'name':'main'},44))

        # Safe branch deletion uses -d semantics; force deletion is unavailable.
        deleted=payload(call(url,'git_branch_delete',{**cwd,'name':'topic'},50)); assert deleted['branch']=='topic',deleted
        expect_error(url,'git_branch_delete',{**cwd,'name':'rebase-topic'},'unmerged branch delete',51)

        # Repo-configured executable filter/merge drivers fail before Git mutation.
        canary=os.path.join(owner,'EXECUTED-CANARY')
        host_run(repo,'config','filter.evil.clean',f'sh -c "touch {canary}; cat"')
        open(os.path.join(repo,'.gitattributes'),'w').write('*.evil filter=evil\n')
        open(os.path.join(repo,'sample.evil'),'w').write('x\n')
        expect_error(url,'git_stage',{**cwd,'paths':['sample.evil']},'custom clean filter denied',60)
        assert not os.path.exists(canary),canary
        host_run(repo,'config','--unset','filter.evil.clean')
        host_run(repo,'config','merge.evil.driver',f'sh -c "touch {canary}; exit 1"')
        expect_error(url,'git_branch_create',{**cwd,'name':'blocked-by-driver'},'custom merge driver config denied',61)
        assert not os.path.exists(canary),canary

        print('040B local Git mutation/conflict acceptance: PASS')
    finally:
        proc.terminate()
        try: proc.wait(timeout=3)
        except subprocess.TimeoutExpired: proc.kill(); proc.wait(timeout=3)
PY
