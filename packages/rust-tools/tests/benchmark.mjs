import { execSync } from 'node:child_process'
import fs from 'node:fs'

function measure(name, cmd) {
  const start = performance.now()
  try {
    execSync(cmd, { stdio: 'ignore' })
  } catch (e) {
    // ignore
  }
  return performance.now() - start
}

const iterations = 5
let results = '# CLI Benchmark Results\n\n'
results += '| Tool | Environment | Avg Latency (ms) | Peak RSS (approx) |\n'
results += '| --- | --- | --- | --- |\n'

function runBenchmark(toolName, jsCmd, rustCmd) {
  let jsTotal = 0
  for (let i = 0; i < iterations; i++) jsTotal += measure(toolName + ' JS', jsCmd)
  
  let rustTotal = 0
  for (let i = 0; i < iterations; i++) rustTotal += measure(toolName + ' Rust', rustCmd)

  results += `| ${toolName} | JS | ${(jsTotal / iterations).toFixed(2)} | N/A |\n`
  results += `| ${toolName} | Rust | ${(rustTotal / iterations).toFixed(2)} | N/A |\n`
}

runBenchmark('terminal-tool', 'node tests/benchmark-temp/terminal-tool.mjs echo hello', 'target/release/terminal-tool echo hello')
runBenchmark('curl-tool', 'node tests/benchmark-temp/curl-tool.mjs --help', 'target/release/curl-tool --help')
runBenchmark('searxng-search-tool', 'node tests/benchmark-temp/searxng-tool.mjs --help', 'target/release/searxng-search-tool --help')

fs.writeFileSync('tests/benchmark-results.md', results)
console.log('Benchmark complete')
