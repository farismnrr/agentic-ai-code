---
name: searxng-search-tool
description: Standalone LangChain tool for searching via SearxNG.
license: MIT
---

# @ai-code/searxng-search-tool

A standalone LangChain tool that performs searches against a SearxNG instance.

## Usage

### In Code

```ts
import { createSearxngSearchTool } from '@ai-code/searxng-search-tool'

// Instantiate the tool by providing the base URL of your SearxNG instance
const searchTool = createSearxngSearchTool({
  baseUrl: 'http://127.0.0.1:8888'
})

// Execute a search query
const results = await searchTool.invoke({ query: 'latest tech news' })
console.log(results)
```

### CLI

Run searches directly from the terminal. If you don't provide a `--base-url`, it defaults to `http://127.0.0.1:8888`.

**Basic Search:**
```bash
npx @ai-code/searxng-search-tool "how to build a web scraper"
```

**Search with Custom Base URL:**
```bash
npx @ai-code/searxng-search-tool "Nuxt 3 documentation" --base-url https://searx.my-domain.com
```
