#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/Cargo.toml"
command -v cargo >/dev/null
command -v python3 >/dev/null
command -v bwrap >/dev/null
command -v rg >/dev/null
RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked --bin ai-tools
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request
RELAY, ROOT = sys.argv[1:]
PROTOCOL="2026-07-28"; ORIGIN="http://localhost:3333"
def free_port():
    with socket.socket() as s: s.bind(("127.0.0.1",0)); return s.getsockname()[1]
def req(url, headers, body):
    r=urllib.request.Request(url,data=json.dumps(body).encode(),headers=headers,method="POST")
    try:
        with urllib.request.urlopen(r,timeout=10) as x: return x.status,json.loads(x.read())
    except urllib.error.HTTPError as e: return e.code,json.loads(e.read())
def headers(method,name=None):
    h={"Content-Type":"application/json","Origin":ORIGIN,"MCP-Protocol-Version":PROTOCOL,"Mcp-Method":method}
    if name: h["Mcp-Name"]=name
    return h
def mcp(method,params,i=1):
    params=dict(params); params["_meta"]={"io.modelcontextprotocol/protocolVersion":PROTOCOL,"io.modelcontextprotocol/clientCapabilities":{}}
    return {"jsonrpc":"2.0","id":i,"method":method,"params":params}
def wait(port,p):
    for _ in range(100):
        if p.poll() is not None: raise AssertionError(p.stderr.read())
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/health",timeout=1) as x:
                if x.status==200:return
        except Exception: pass
        time.sleep(.05)
    raise AssertionError("relay health timeout")
def call(url,args,i=10):
    st,b=req(url,headers("tools/call","text_search"),mcp("tools/call",{"name":"text_search","arguments":args},i))
    if st!=200: raise AssertionError((st,b))
    r=b["result"]
    if r["isError"]: return r,None
    text=next(x["text"] for x in r["content"] if x.get("type")=="text")
    return r,json.loads(text)
def expect_error(url,args,label):
    st,b=req(url,headers("tools/call","text_search"),mcp("tools/call",{"name":"text_search","arguments":args},77))
    if st==400:
        assert b["error"]["code"]==-32602,(label,b); return
    assert st==200,(label,st,b); assert b["result"]["isError"] is True,(label,b)
with tempfile.TemporaryDirectory(prefix="relay-text-search-") as base:
    ws=os.path.join(base,"workspace"); ext=os.path.join(base,"external")
    os.makedirs(os.path.join(ws,"src")); os.makedirs(os.path.join(ws,"ignored")); os.makedirs(os.path.join(ws,".git")); os.makedirs(ext)
    open(os.path.join(ws,"src","a.rs"),"w").write("Needle here\nneedle lower\nαβ Needle unicode\n")
    open(os.path.join(ws,"src","b.txt"),"w").write("Needle text\nregex-123\n")
    open(os.path.join(ws,"ignored","skip.rs"),"w").write("Needle ignored\n")
    open(os.path.join(ws,".gitignore"),"w").write("ignored/\n")
    open(os.path.join(ws,"binary.bin"),"wb").write(b"\x00Needle\x00")
    open(os.path.join(ext,"CANARY-EXTERNAL-038.txt"),"w").write("Needle external\n")
    os.symlink(os.path.join(ext,"CANARY-EXTERNAL-038.txt"),os.path.join(ws,"external-link.txt"))
    os.makedirs(os.path.join(ws,"many"))
    for i in range(120): open(os.path.join(ws,"many",f"m{i:03}.txt"),"w").write("Needle\n")
    port=free_port(); p=subprocess.Popen([RELAY,"relay","--port",str(port),"--dir",ws,"--execution-root",ws,"--origin",ORIGIN,"--mode","local"],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:
        wait(port,p); url=f"http://127.0.0.1:{port}/mcp"
        st,b=req(url,headers("tools/list"),mcp("tools/list",{})); assert st==200
        t=next(x for x in b["result"]["tools"] if x["name"]=="text_search")
        assert t["inputSchema"]["additionalProperties"] is False
        assert t["inputSchema"]["properties"]["max_results"]["maximum"]==100
        assert t["annotations"]=={"readOnlyHint":True,"destructiveHint":False,"idempotentHint":True,"openWorldHint":False}
        assert t["securitySchemes"]==[{"type":"oauth2","scopes":["relay.coding"]}]
        _,x=call(url,{"query":"Needle","cwd":"src"}); assert [m["path"] for m in x["matches"]]==["a.rs", "a.rs", "b.txt"] and x["count"]==3
        assert all(len(m["preview"].encode())<=1024 for m in x["matches"])
        _,x=call(url,{"query":"needle","cwd":"src","case_sensitive":False}); assert x["count"]==4
        _,x=call(url,{"query":"regex-[0-9]+","regex":True}); assert any(m["path"]=="src/b.txt" for m in x["matches"])
        _,x=call(url,{"query":"Needle","glob":"*.rs"}); assert all(m["path"].endswith(".rs") for m in x["matches"]); assert not any("ignored" in m["path"] for m in x["matches"])
        _,x=call(url,{"query":"αβ"}); assert x["count"]==1
        _,x=call(url,{"query":"Needle","cwd":"many","max_results":5}); assert x["count"]==5 and x["truncated"] is True
        _,x=call(url,{"query":"Needle"}); assert not any("external-link" in m["path"] or "CANARY-EXTERNAL" in m["path"] for m in x["matches"])
        assert not any(m["path"]=="binary.bin" for m in x["matches"])
        expect_error(url,{"query":"(","regex":True},"invalid regex")
        expect_error(url,{"query":"Needle","cwd":"../"},"cwd traversal")
        expect_error(url,{"query":"Needle","cwd":ext},"external cwd")
        expect_error(url,{"query":"x"*4097},"oversized query")
        expect_error(url,{"query":"Needle","glob":"x"*4097},"oversized glob")
        st,b=req(url,headers("tools/call","text_search"),mcp("tools/call",{"name":"text_search","arguments":{"query":"Needle","extra":True}},90)); assert st==400 and b["error"]["code"]==-32602
        open(os.path.join(ws,"huge.txt"),"w").write("x"*5000+"Needle"+"y"*5000+"\n")
        expect_error(url,{"query":"Needle","cwd":".","glob":"huge.txt"},"giant matching line")
        print("text_search MCP acceptance: PASS")
    finally:
        p.terminate()
        try:p.wait(timeout=5)
        except subprocess.TimeoutExpired:p.kill()
PY
