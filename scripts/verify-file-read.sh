#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json,os,socket,subprocess,sys,tempfile,time,urllib.request,urllib.error
relay,root=sys.argv[1:]; P="2026-07-28"; O="http://localhost:3333"
def port():
 s=socket.socket(); s.bind(("127.0.0.1",0)); x=s.getsockname()[1]; s.close(); return x
def post(url,method,name,args,i=1):
 h={"Content-Type":"application/json","Origin":O,"MCP-Protocol-Version":P,"Mcp-Method":method}
 if name:h["Mcp-Name"]=name
 p={"_meta":{"io.modelcontextprotocol/protocolVersion":P,"io.modelcontextprotocol/clientCapabilities":{}}}
 if method=="tools/call": p.update({"name":name,"arguments":args})
 b={"jsonrpc":"2.0","id":i,"method":method,"params":p}
 r=urllib.request.Request(url,data=json.dumps(b).encode(),headers=h,method="POST")
 try:
  with urllib.request.urlopen(r,timeout=10) as x:return x.status,json.loads(x.read())
 except urllib.error.HTTPError as e:return e.code,json.loads(e.read())
def call(url,args):
 st,b=post(url,"tools/call","file_read",args); assert st==200,(st,b); r=b["result"]
 if r["isError"]: return None
 return json.loads(next(x["text"] for x in r["content"] if x.get("type")=="text"))
def err(url,args):
 st,b=post(url,"tools/call","file_read",args); assert st in (200,400),(st,b)
 assert (st==400 and b["error"]["code"]==-32602) or b["result"]["isError"] is True,b
with tempfile.TemporaryDirectory(prefix="relay-file-read-") as base:
 ws=os.path.join(base,"ws"); ext=os.path.join(base,"ext"); os.makedirs(os.path.join(ws,"dir")); os.makedirs(ext)
 open(os.path.join(ws,"empty.txt"),"w").close(); open(os.path.join(ws,"small.txt"),"w",encoding="utf8").write("one\ntwo\nтри\nfour\n")
 open(os.path.join(ws,"bad.bin"),"wb").write(b"ok\n\xffbad\n")
 open(os.path.join(ws,"huge.txt"),"w").write("x"*(64*1024+1)+"\n")
 open(os.path.join(ext,"CANARY-EXTERNAL-038.txt"),"w").write("secret")
 os.symlink(os.path.join(ext,"CANARY-EXTERNAL-038.txt"),os.path.join(ws,"external-link.txt"))
 open(os.path.join(ws,"contained.txt"),"w").write("inside\n"); os.symlink("contained.txt",os.path.join(ws,"contained-link.txt"))
 pnum=port(); p=subprocess.Popen([relay,"relay","--port",str(pnum),"--dir",ws,"--execution-root",ws,"--origin",O,"--mode","local"],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  for _ in range(100):
   try:
    with urllib.request.urlopen(f"http://127.0.0.1:{pnum}/health",timeout=1) as h:
     if h.status==200:break
   except Exception:time.sleep(.05)
  url=f"http://127.0.0.1:{pnum}/mcp"; st,b=post(url,"tools/list",None,{}) ; assert st==200
  t=next(x for x in b["result"]["tools"] if x["name"]=="file_read"); assert t["inputSchema"]["additionalProperties"] is False and t["annotations"]["readOnlyHint"] is True
  x=call(url,{"path":"empty.txt"}); assert x["content"]=="" and x["end_line"] is None and x["truncated"] is False
  x=call(url,{"path":"small.txt","offset_line":2,"limit_lines":2}); assert x["content"]=="two\nтри\n" and x["start_line"]==2 and x["end_line"]==3 and x["truncated"] is True
  x=call(url,{"path":"small.txt","offset_line":99}); assert x["content"]=="" and x["end_line"] is None
  x=call(url,{"path":"contained-link.txt"}); assert x["content"]=="inside\n"
  err(url,{"path":"bad.bin"}); err(url,{"path":"huge.txt"}); err(url,{"path":"dir"}); err(url,{"path":"missing.txt"}); err(url,{"path":"external-link.txt"}); err(url,{"path":"../x"}); err(url,{"path":"/etc/passwd"})
  st,b=post(url,"tools/call","file_read",{"path":"small.txt","extra":1}); assert st==400 and b["error"]["code"]==-32602
  print("file_read MCP acceptance: PASS")
 finally:
  p.terminate(); p.wait(timeout=5)
PY
