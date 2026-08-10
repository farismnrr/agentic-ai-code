import esbuild from 'esbuild'
import path from 'node:path'

async function build() {
  await esbuild.build({
    entryPoints: [path.resolve('bin/cli.mjs')],
    bundle: true,
    platform: 'node',
    target: 'node20',
    format: 'cjs',
    outfile: path.resolve('dist/bundle.cjs'),
    external: []
  })
  console.log('[relay-agent] Bundled CLI entrypoint to dist/bundle.cjs')
}

build().catch((err) => {
  console.error(err)
  process.exit(1)
})
