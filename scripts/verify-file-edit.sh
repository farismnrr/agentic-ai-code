#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json,os,socket,stat,subprocess,sys,tempfile,time,threading,urllib.request,urllib.error
relay,root=sys.argv[1:]; P="2026-07-28"; O="http://localhost:3333"
def port(): s=socket.socket(); s.bind(("127.0.0.1",0)); p=s.getsockname()[1]; s.close(); return p
def post(url,args,i=1):
 h={"Content-Type":"application/json","Origin":O,"MCP-Protocol-Version":P,"Mcp-Method":"tools/call","Mcp-Name":"file_edit"}; params={"name":"file_edit","arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":P,"io.modelcontextprotocol/clientCapabilities":{}}}; body={"jsonrpc":"2.0","id":i,"method":"tools/call","params":params}; r=urllib.request.Request(url,data=json.dumps(body).encode(),headers=h,method="POST")
 try:
  with urllib.request.urlopen(r,timeout=10) as x:return x.status,json.loads(x.read())
 except urllib.error.HTTPError as e:return e.code,json.loads(e.read())
def call(url,args):
 st,b=post(url,args); assert st==200,(st,b); r=b["result"]
 if r["isError"]: return None
 return json.loads(next(x["text"] for x in r["content"] if x.get("type")=="text"))
def err(url,args):
 st,b=post(url,args); assert st in (200,400),(st,b); assert (st==400 and b["error"]["code"]==-32602) or b["result"]["isError"] is True,b
with tempfile.TemporaryDirectory(prefix="relay-file-edit-") as base:
 ws=os.path.join(base,"ws"); ext=os.path.join(base,"ext"); os.makedirs(ws); os.makedirs(ext)
 def put(name,text,mode=0o640):
  p=os.path.join(ws,name); open(p,"w",encoding="utf8").write(text); os.chmod(p,mode); return p
 one=put("one.txt","alpha βeta gamma\n"); multi=put("multi.txt","x x x\n"); amb=put("amb.txt","dup dup\n"); same=put("same.txt","stable\n"); batch=put("batch.txt","first = 1\nsecond = 2\nthird = 3\n"); preflight=put("preflight.txt","keep this\nchange this\n"); over=put("over.txt","a"*(1024*1024+1))
 canary=os.path.join(ext,"CANARY-EXTERNAL-038.txt"); open(canary,"w").write("external")
 os.symlink(canary,os.path.join(ws,"external-link.txt")); os.symlink(ext,os.path.join(ws,"external-parent"))
 pnum=port(); p=subprocess.Popen([relay,"relay","--port",str(pnum),"--dir",ws,"--execution-root",ws,"--origin",O,"--mode","local"],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  for _ in range(100):
   try:
    with urllib.request.urlopen(f"http://127.0.0.1:{pnum}/health",timeout=1) as h:
     if h.status==200: break
   except Exception: time.sleep(.05)
  url=f"http://127.0.0.1:{pnum}/mcp"
  before=stat.S_IMODE(os.stat(one).st_mode); x=call(url,{"path":"one.txt","old_text":"βeta","new_text":"BETA"}); assert x=={"path":"one.txt","replacements":1,"changed":True}; assert open(one).read()=="alpha BETA gamma\n"; assert stat.S_IMODE(os.stat(one).st_mode)==before
  x=call(url,{"path":"multi.txt","old_text":"x","new_text":"y","replace_all":True}); assert x["replacements"]==3 and open(multi).read()=="y y y\n"
  x=call(url,{"path":"batch.txt","edits":[{"old_text":"first = 1","new_text":"first = 10"},{"old_text":"third = 3","new_text":"third = 30"}]}); assert x=={"path":"batch.txt","replacements":2,"changed":True}; assert open(batch).read()=="first = 10\nsecond = 2\nthird = 30\n"
  original=open(preflight).read(); err(url,{"path":"preflight.txt","edits":[{"old_text":"keep this","new_text":"changed"},{"old_text":"missing","new_text":"still missing"}]}); assert open(preflight).read()==original
  original=open(batch).read(); err(url,{"path":"batch.txt","edits":[{"old_text":"first = 10","new_text":"x"},{"old_text":"first = 10\nsecond","new_text":"y"}]}); assert open(batch).read()==original
  original=open(amb).read(); err(url,{"path":"amb.txt","old_text":"dup","new_text":"z"}); assert open(amb).read()==original
  err(url,{"path":"amb.txt","old_text":"missing","new_text":"z"}); assert open(amb).read()==original
  x=call(url,{"path":"same.txt","old_text":"stable","new_text":"stable"}); assert x["changed"] is False and open(same).read()=="stable\n"
  x=call(url,{"path":"one.txt","old_text":"BETA","new_text":""}); assert x["changed"] is True and open(one).read()=="alpha  gamma\n"
  err(url,{"path":"over.txt","old_text":"a","new_text":"b","replace_all":True})
  err(url,{"path":"external-link.txt","old_text":"external","new_text":"pwn"}); assert open(canary).read()=="external"
  err(url,{"path":"external-parent/CANARY-EXTERNAL-038.txt","old_text":"external","new_text":"pwn"}); assert open(canary).read()=="external"
  err(url,{"path":"../outside.txt","old_text":"x","new_text":"y"}); err(url,{"path":"/etc/passwd","old_text":"root","new_text":"x"})
  race=os.path.join(ws,"race.txt"); backup=os.path.join(ws,"race-backup.txt"); open(race,"w").write("safe token\n")
  running=True
  def swap():
   while running:
    try:
     os.rename(race,backup); os.symlink(canary,race); os.unlink(race); os.rename(backup,race)
    except FileNotFoundError: pass
  th=threading.Thread(target=swap); th.start()
  try:
   for _ in range(5):
    call(url,{"path":"race.txt","old_text":"token","new_text":"token"}); time.sleep(.08)
  finally:
   running=False; th.join(timeout=5)
  if os.path.islink(race): os.unlink(race)
  if os.path.exists(backup): os.rename(backup,race)
  assert open(canary).read()=="external"
  st,b=post(url,{"path":"one.txt","old_text":"x","new_text":"y","extra":1},9); assert st==400 and b["error"]["code"]==-32602
  print("file_edit MCP acceptance: PASS")
 finally:
  p.terminate(); p.wait(timeout=5)
PY
