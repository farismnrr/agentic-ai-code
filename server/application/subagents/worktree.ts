/* eslint-disable @stylistic/max-statements-per-line */
import { execFile } from 'node:child_process'
import { mkdir, readFile, realpath, unlink, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { promisify } from 'node:util'
import { randomUUID } from 'node:crypto'

const execFileAsync = promisify(execFile)
const OWNER_FILE = '.ai-code-worktree-owner.json'
const MAX_GIT_OUTPUT = 4096

export interface WorktreeOwner { task_id: string, parent_session_id: string, user_id: string, repository_identity: string, repository_root: string, branch: string, base_commit: string, worktree_path: string, owner_metadata_path: string }
export interface WorktreeAllocation { owner: WorktreeOwner, root: string }
export interface GitRunner { (args: string[], cwd: string): Promise<string> }

function bound(value: string) { return value.replaceAll(/\p{Cc}/gu, ' ').slice(0, MAX_GIT_OUTPUT) }
function contained(root: string, candidate: string) {
  const rel = relative(root, candidate)
  return rel === '' || (!rel.startsWith('..') && !rel.startsWith('/') && !rel.includes('\\'))
}

export class WorktreeAllocator {
  private readonly runGit: GitRunner
  constructor(runGit: GitRunner = defaultGit) { this.runGit = runGit }

  async allocate(input: { repositoryRoot: string, worktreeRoot: string, taskId: string, parentSessionId: string, userId: string, baseRef?: string }): Promise<WorktreeAllocation> {
    const repositoryRoot = await realpath(input.repositoryRoot)
    const worktreeRoot = await realpathOrCreate(input.worktreeRoot)
    if (!contained(repositoryRoot, worktreeRoot)) throw new Error('worktree root escapes repository boundary')
    const origin = bound(await this.runGit(['config', '--get', 'remote.origin.url'], repositoryRoot))
    if (!origin) throw new Error('repository identity is unavailable')
    const baseCommit = bound(await this.runGit(['rev-parse', '--verify', input.baseRef ?? 'HEAD'], repositoryRoot)).trim()
    const status = await this.runGit(['status', '--porcelain'], repositoryRoot)
    if (status.trim()) throw new Error('writer background task requires a clean parent checkout')
    const safeId = input.taskId.replaceAll(/[^a-zA-Z0-9-]/g, '').slice(0, 48)
    if (!safeId) throw new Error('task identity is invalid')
    const branch = `ai-code/background/${safeId}-${randomUUID().slice(0, 8)}`
    const path = resolve(worktreeRoot, safeId)
    if (!contained(worktreeRoot, path) || existsSync(path)) throw new Error('task worktree path collision')
    await mkdir(dirname(path), { recursive: true })
    await this.runGit(['branch', branch, input.baseRef ?? 'HEAD'], repositoryRoot)
    try {
      await this.runGit(['worktree', 'add', path, branch], repositoryRoot)
      const owner: WorktreeOwner = { task_id: input.taskId, parent_session_id: input.parentSessionId, user_id: input.userId, repository_identity: origin, repository_root: repositoryRoot, branch, base_commit: baseCommit, worktree_path: path, owner_metadata_path: join(worktreeRoot, `${safeId}${OWNER_FILE}`) }
      await writeFile(owner.owner_metadata_path, JSON.stringify(owner), { encoding: 'utf8', flag: 'wx' })
      return { owner, root: path }
    } catch (error) {
      // Best-effort rollback only for the branch/path this allocator just created.
      await this.runGit(['branch', '-D', branch], repositoryRoot).catch(() => {})
      await unlink(join(worktreeRoot, `${safeId}${OWNER_FILE}`)).catch(() => {})
      throw error
    }
  }

  async inspect(owner: WorktreeOwner): Promise<{ owned: boolean, clean: boolean, uniqueCommits: boolean }> {
    const path = await realpath(owner.worktree_path)
    if (path !== resolve(owner.worktree_path)) throw new Error('worktree path canonicalization changed')
    const metadata = JSON.parse(await readFile(owner.owner_metadata_path, 'utf8')) as WorktreeOwner
    const same = JSON.stringify(metadata) === JSON.stringify(owner)
    if (!same) return { owned: false, clean: false, uniqueCommits: true }
    const status = await this.runGit(['status', '--porcelain'], path)
    const unique = (await this.runGit(['rev-list', '--count', `${owner.base_commit}..HEAD`], path).catch(() => '1')).trim() !== '0'
    return { owned: true, clean: !status.trim(), uniqueCommits: unique }
  }

  async evidence(owner: WorktreeOwner) {
    const status = bound(await this.runGit(['status', '--short'], owner.worktree_path))
    const diff = bound(await this.runGit(['diff', '--stat'], owner.worktree_path))
    const commits = bound(await this.runGit(['log', '--oneline', `${owner.base_commit}..HEAD`], owner.worktree_path))
    return { status, diff, commits }
  }

  async identity(owner: WorktreeOwner) {
    const inspection = await this.inspect(owner)
    if (!inspection.owned) throw new Error('writer worktree ownership changed')
    const headCommit = bound(await this.runGit(['rev-parse', '--verify', 'HEAD'], owner.worktree_path)).trim()
    if (!/^[0-9a-f]{40}$/i.test(headCommit)) throw new Error('writer head identity is invalid')
    return { branch: owner.branch, base_commit: owner.base_commit, head_commit: headCommit, dirty: !inspection.clean }
  }

  async dispose(owner: WorktreeOwner): Promise<boolean> {
    const inspection = await this.inspect(owner)
    if (!inspection.owned || !inspection.clean || inspection.uniqueCommits) return false
    const path = await realpath(owner.worktree_path)
    await this.runGit(['worktree', 'remove', path], owner.repository_root)
    await this.runGit(['branch', '-D', owner.branch], owner.repository_root)
    await unlink(owner.owner_metadata_path)
    return true
  }
}

async function realpathOrCreate(path: string) {
  await mkdir(path, { recursive: true })
  return realpath(path)
}
async function defaultGit(args: string[], cwd: string) {
  const env = { ...process.env, GIT_DIR: undefined, GIT_WORK_TREE: undefined, GIT_INDEX_FILE: undefined, GIT_COMMON_DIR: undefined }
  const result = await execFileAsync('git', args, { cwd, env, maxBuffer: 64 * 1024 })
  return bound(result.stdout)
}
