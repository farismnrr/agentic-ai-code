#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json,os,socket,stat,subprocess,sys,tempfile,time,threading,urllib.request,urllib.error
relay,root=sys.argv[1:]; P="2026-07-28"; O="http://localhost:3333"
def port(): s=socket.socket(); s.bind(("127.0.0.1",0)); p=s.getsockname()[1]; s.close(); return p
def decode(raw):
 if not raw:return {"_empty":True}
 try:return json.loads(raw)
 except Exception:return {"_raw":raw.decode("utf-8","replace")}
def post(url,args,i=1):
 h={"Content-Type":"application/json","Origin":O,"MCP-Protocol-Version":P,"Mcp-Method":"tools/call","Mcp-Name":"file_write"}; params={"name":"file_write","arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":P,"io.modelcontextprotocol/clientCapabilities":{}}}; body={"jsonrpc":"2.0","id":i,"method":"tools/call","params":params}; r=urllib.request.Request(url,data=json.dumps(body).encode(),headers=h,method="POST")
 try:
  with urllib.request.urlopen(r,timeout=10) as x:
   raw=x.read(); return x.status,decode(raw)
 except urllib.error.HTTPError as e:
  raw=e.read(); return e.code,decode(raw)
def call(url,args):
 st,b=post(url,args); assert st==200,(st,b); r=b["result"]
 if r["isError"]: return None
 return json.loads(next(x["text"] for x in r["content"] if x.get("type")=="text"))
def err(url,args):
 st,b=post(url,args); assert st in (200,400,413),(args,st,b); assert st==413 or (st==400 and b.get("error",{}).get("code")==-32602) or b.get("result",{}).get("isError") is True,(args,b)
with tempfile.TemporaryDirectory(prefix="relay-file-write-") as base:
 ws=os.path.join(base,"ws"); ext=os.path.join(base,"ext"); os.makedirs(os.path.join(ws,"sub")); os.makedirs(ext)
 existing=os.path.join(ws,"existing.txt"); open(existing,"w").write("old\n"); os.chmod(existing,0o640)
 canary=os.path.join(ext,"CANARY-EXTERNAL-038.txt"); open(canary,"w").write("external")
 os.symlink(ext,os.path.join(ws,"external-parent")); os.symlink(canary,os.path.join(ws,"external-link.txt"))
 os.makedirs(os.path.join(ws,"contained-dir")); os.symlink("contained-dir",os.path.join(ws,"contained-parent-link"))
 pnum=port(); p=subprocess.Popen([relay,"relay","--port",str(pnum),"--dir",ws,"--execution-root",ws,"--origin",O,"--mode","local"],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  for _ in range(100):
   try:
    with urllib.request.urlopen(f"http://127.0.0.1:{pnum}/health",timeout=1) as h:
     if h.status==200:break
   except Exception:time.sleep(.05)
  url=f"http://127.0.0.1:{pnum}/mcp"
  x=call(url,{"path":"new.txt","content":"héllo\n"}); assert x=={"path":"new.txt","created":True,"overwritten":False,"bytes":len("héllo\n".encode())}; assert open(os.path.join(ws,"new.txt"),encoding="utf8").read()=="héllo\n"; assert stat.S_IMODE(os.stat(os.path.join(ws,"new.txt")).st_mode)==0o644
  err(url,{"path":"existing.txt","content":"nope"}); assert open(existing).read()=="old\n"
  before=stat.S_IMODE(os.stat(existing).st_mode); x=call(url,{"path":"existing.txt","content":"new\n","overwrite":True}); assert x["overwritten"] is True and open(existing).read()=="new\n" and stat.S_IMODE(os.stat(existing).st_mode)==before
  err(url,{"path":"missing/a/b.txt","content":"x"}); assert not os.path.exists(os.path.join(ws,"missing"))
  x=call(url,{"path":"missing/a/b.txt","content":"nested","create_parents":True}); assert x["created"] is True and open(os.path.join(ws,"missing/a/b.txt")).read()=="nested"
  x=call(url,{"path":"sub/../normalized.txt","content":"ok"}); assert open(os.path.join(ws,"normalized.txt")).read()=="ok"
  err(url,{"path":"../../escape.txt","content":"x","create_parents":True}); err(url,{"path":"/etc/escape.txt","content":"x","create_parents":True})
  err(url,{"path":"external-parent/pwn.txt","content":"pwn","create_parents":True}); assert not os.path.exists(os.path.join(ext,"pwn.txt"))
  err(url,{"path":"contained-parent-link/pwn.txt","content":"pwn","create_parents":True}); assert not os.path.exists(os.path.join(ws,"contained-dir/pwn.txt"))
  err(url,{"path":"external-link.txt","content":"pwn","overwrite":True}); assert open(canary).read()=="external"
  err(url,{"path":"oversize.txt","content":"x"*(1024*1024+1)})
  race=os.path.join(ws,"race-create.txt"); running=True
  def create_racer():
   while running:
    try:
     fd=os.open(race,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600); os.write(fd,b"racer"); os.close(fd)
    except FileExistsError: pass
    try: os.unlink(race)
    except FileNotFoundError: pass
  th=threading.Thread(target=create_racer); th.start()
  try:
   for _ in range(5):
    call(url,{"path":"race-create.txt","content":"relay"}); time.sleep(.08)
  finally:
   running=False; th.join(timeout=5)
  if os.path.exists(race):
   assert open(race).read() in ("racer","relay")
  assert open(canary).read()=="external"
  st,b=post(url,{"path":"x","content":"y","extra":1},90); assert st==400 and b["error"]["code"]==-32602
  print("file_write MCP acceptance: PASS")
 finally:
  p.terminate(); p.wait(timeout=5)
PY
