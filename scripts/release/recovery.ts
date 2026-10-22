import assert from 'node:assert/strict'
import { appendFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import { runReleaseCommand } from './commands.ts'
import { logReleaseProgress } from './progress.ts'
import {
  assertCleanReleaseSource,
  readObject,
  validateVersion,
} from './project.ts'

interface RecoverySource {
  runId: number
  tag: string
  revision: string
}

/**
 * Derives the original source from one exact artifact, failing closed on provenance.
 */
export function validateRecoveryProvenance(
  artifactValue: unknown,
  runValue: unknown,
  jobsValue: unknown,
  repository: string,
): RecoverySource {
  const artifact = readObject(artifactValue)
  const origin = readObject(artifact.workflow_run)
  const run = readObject(runValue)
  const owner = readObject(run.repository)
  assert.equal(owner.full_name, repository)
  assert.ok(
    typeof owner.id === 'number' &&
      Number.isSafeInteger(owner.id) &&
      owner.id > 0,
  )
  assert.equal(origin.repository_id, owner.id)
  assert.equal(origin.head_repository_id, owner.id)
  assert.equal(run.path, '.github/workflows/release.yml')
  assert.equal(run.status, 'completed')
  assert.ok(
    typeof origin.id === 'number' &&
      Number.isSafeInteger(origin.id) &&
      origin.id > 0,
  )
  assert.equal(run.id, origin.id)
  assert.equal(run.head_sha, origin.head_sha)
  assert.equal(run.head_branch, origin.head_branch)
  assert.ok(typeof run.head_sha === 'string')
  assert.match(run.head_sha, /^[a-f0-9]{40}$/)
  assert.ok(
    typeof run.head_branch === 'string' && run.head_branch.startsWith('v'),
  )
  validateVersion(run.head_branch.slice(1))
  assert.equal(artifact.expired, false)
  assert.ok(typeof artifact.name === 'string')
  assert.match(artifact.name, /^release-artifacts-[1-9][0-9]*$/)
  assert.ok(Array.isArray(jobsValue))
  const jobs = jobsValue.flatMap(page => {
    const entries = readObject(page).jobs
    assert.ok(Array.isArray(entries))
    return entries.map(readObject)
  })
  for (const name of ['freeze', 'publish-crates'])
    assert.ok(
      jobs.some(job => job.name === name && job.conclusion === 'success'),
      `missing successful ${name}`,
    )
  return { runId: origin.id, tag: run.head_branch, revision: run.head_sha }
}

async function prepareRecovery(artifactId: string): Promise<void> {
  assert.match(artifactId, /^[1-9][0-9]*$/)
  assert.equal(process.env.GITHUB_REF_TYPE, 'tag')
  assert.match(process.env.GITHUB_REF_NAME ?? '', /^v-recovery-[\w.-]+$/)
  const repository = process.env.GH_REPO ?? ''
  const branch = process.env.DEFAULT_BRANCH ?? ''
  const output = process.env.GITHUB_OUTPUT
  assert.match(repository, /^[\w.-]+\/[\w.-]+$/)
  assert.ok(branch && output, 'missing default branch or GitHub step output')
  assertCleanReleaseSource()
  const command = (executable: string, args: string[]): Promise<string> =>
    runReleaseCommand(executable, args, {
      label: `${executable} ${args.join(' ')}`,
      captureOutput: true,
      timeoutMs: 60_000,
    })
  const api = async (path: string, ...args: string[]): Promise<unknown> =>
    JSON.parse(
      await command('gh', [
        'api',
        `repos/${repository}/actions/${path}`,
        ...args,
      ]),
    )
  await command('git', ['check-ref-format', `refs/heads/${branch}`])
  await command('git', [
    'merge-base',
    '--is-ancestor',
    'HEAD',
    `origin/${branch}`,
  ])
  const artifact = readObject(await api(`artifacts/${artifactId}`))
  assert.equal(String(artifact.id), artifactId)
  const origin = readObject(artifact.workflow_run)
  assert.ok(
    typeof origin.id === 'number' &&
      Number.isSafeInteger(origin.id) &&
      origin.id > 0,
  )
  const source = validateRecoveryProvenance(
    artifact,
    await api(`runs/${origin.id}`),
    await api(
      `runs/${origin.id}/jobs?filter=all&per_page=100`,
      '--paginate',
      '--slurp',
    ),
    repository,
  )
  assert.equal(
    (await command('git', ['rev-parse', `${source.tag}^{commit}`])).trim(),
    source.revision,
    'original version tag moved',
  )
  await command('git', ['merge-base', '--is-ancestor', source.revision, 'HEAD'])
  logReleaseProgress(
    `Recovering ${source.tag} from artifact ${artifactId} (${artifact.digest})`,
  )
  await command('git', [
    'worktree',
    'add',
    '--detach',
    'target/recovery-source',
    source.revision,
  ])
  appendFileSync(output, `run-id=${source.runId}\n`)
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const { values } = parseArgs({ options: { artifact: { type: 'string' } } })
  assert.ok(values.artifact, 'usage: recovery.ts --artifact ARTIFACT_ID')
  await prepareRecovery(values.artifact)
}
