#!/bin/bash
./target/release/relay-agent --port 8080 --origin "http://localhost:3000" &
PID=$!
sleep 2

echo "--- 1. Testing server/discover ---"
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost:3000" \
  -H "Mcp-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: server/discover" \
  -d '{"jsonrpc": "2.0", "method": "server/discover", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientCapabilities": {}}}, "id": 1}' | head -c 500
echo

echo "--- 2. Testing tools/list ---"
curl -s -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost:3000" \
  -H "Mcp-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/list" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientCapabilities": {}}}, "id": 2}' | head -c 500
echo

echo "--- 3. Stop via CLI ---"
./target/release/relay-agent stop --port 8080
kill $PID 2>/dev/null
