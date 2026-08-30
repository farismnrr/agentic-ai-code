import assert from 'node:assert/strict'
import test from 'node:test'
import { createWorkspaceUseCases } from '../../server/application/workspaces.ts'
import { createActivityUseCases, type ActivityPort } from '../../server/application/activity.ts'

test('workspace active selection verifies ownership before mutation', async () => {
  const calls: string[] = []
  const useCases = createWorkspaceUseCases({
    list: async () => [],
    create: async () => ({ id: 'x' }),
    update: async () => ({ id: 'x' }),
    remove: async () => ({ ok: true as const }),
    find: async (userId, id) => {
      calls.push(`find:${userId}:${id}`)
      if (userId !== 'owner') throw new Error('not found')
      return { id }
    },
    setActive: async (userId, id) => { calls.push(`set:${userId}:${id}`) }
  })

  await useCases.setActive('owner', 'workspace-1')
  assert.deepEqual(calls, ['find:owner:workspace-1', 'set:owner:workspace-1'])

  calls.length = 0
  await assert.rejects(() => useCases.setActive('attacker', 'workspace-1'))
  assert.deepEqual(calls, ['find:attacker:workspace-1'])
})

test('clearing active workspace does not invent an ownership lookup', async () => {
  let findCalls = 0
  let selected: string | null | undefined
  const useCases = createWorkspaceUseCases({
    list: async () => [],
    create: async () => ({}),
    update: async () => ({}),
    remove: async () => ({ ok: true as const }),
    find: async () => {
      findCalls++
      return {}
    },
    setActive: async (_userId, id) => { selected = id }
  })
  await useCases.setActive('owner', null)
  assert.equal(findCalls, 0)
  assert.equal(selected, null)
})

test('activity list limits are bounded before reaching persistence', async () => {
  const seen: number[] = []
  const stub = async () => {
    throw new Error('unused')
  }
  const port: ActivityPort = {
    enroll: stub,
    listSources: stub,
    revoke: stub,
    bind: stub,
    ingest: stub,
    list: async (_userId, _workspaceId, options) => {
      seen.push(options.limit)
      return { items: [] }
    },
    detail: stub,
    diff: stub,
    clear: stub,
    retain: stub
  }
  const useCases = createActivityUseCases(port)
  await useCases.list('owner', 'workspace-1', { limit: 0 })
  await useCases.list('owner', 'workspace-1', { limit: 10_000 })
  await useCases.list('owner', 'workspace-1', {})
  assert.deepEqual(seen, [1, 100, 50])
})
