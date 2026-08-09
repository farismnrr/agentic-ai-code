---
name: curl-tool
description: Standalone LangChain tool for fetching URLs.
license: MIT
---

# @ai-code/curl-tool

A standalone LangChain tool that performs an HTTP fetch, guarded by an injected SSRF validation function.

## Usage

### In Code

```ts
import { createCurlTool } from '@ai-code/curl-tool'

const myCurlTool = createCurlTool({
  assertSafeUrl: async (url, context) => {
    // Implement your own security guard
    if (url.hostname === 'localhost') throw new Error('Blocked')
  }
})

// Basic GET
const getResult = await myCurlTool.invoke({ url: 'https://example.com' })

// POST request with headers and body
const postResult = await myCurlTool.invoke({
  url: 'https://api.example.com/data',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer token123'
  },
  body: JSON.stringify({ key: 'value' })
})
```

### CLI

The CLI provides a way to run requests directly. Use `--no-guard` to explicitly bypass SSRF validations when running locally.

**Basic GET Request:**
```bash
npx @ai-code/curl-tool https://example.com --no-guard
```

**POST Request with Data and Headers:**
```bash
npx @ai-code/curl-tool https://httpbin.org/post \
  --request POST \
  --header "Content-Type: application/json" \
  --header "Authorization: Bearer my-token" \
  --data '{"hello": "world"}' \
  --no-guard
```
