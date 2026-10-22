import assert from 'node:assert/strict'
import { test } from 'node:test'
import { validateRecoveryProvenance } from '../../scripts/release/recovery.ts'

function recoveryFixture() {
  const revision = 'a'.repeat(40)
  return {
    artifact: {
      expired: false,
      name: 'release-artifacts-1',
      workflow_run: {
        id: 100,
        repository_id: 1,
        head_repository_id: 1,
        head_branch: 'v0.1.0-alpha.5',
        head_sha: revision,
      },
    },
    run: {
      id: 100,
      repository: { id: 1, full_name: 'ntnyq/gdfmt' },
      path: '.github/workflows/release.yml',
      status: 'completed',
      head_sha: revision,
      head_branch: 'v0.1.0-alpha.5',
    },
    jobs: [
      { jobs: [{ name: 'freeze', conclusion: 'success' }] },
      { jobs: [{ name: 'publish-crates', conclusion: 'success' }] },
    ],
  }
}

test('recovery derives the original run and version tag from the artifact, accepting paginated successful jobs', () => {
  const { artifact, run, jobs } = recoveryFixture()
  assert.deepEqual(
    validateRecoveryProvenance(artifact, run, jobs, 'ntnyq/gdfmt'),
    {
      runId: 100,
      tag: 'v0.1.0-alpha.5',
      revision: 'a'.repeat(40),
    },
  )
})

test('recovery rejects expired, foreign, mismatched and unsuccessful provenance', () => {
  const reject = (
    mutate: (fixture: ReturnType<typeof recoveryFixture>) => void,
  ): void => {
    const fixture = recoveryFixture()
    mutate(fixture)
    assert.throws(() =>
      validateRecoveryProvenance(
        fixture.artifact,
        fixture.run,
        fixture.jobs,
        'ntnyq/gdfmt',
      ),
    )
  }
  reject(({ artifact }) => {
    artifact.expired = true
  })
  reject(({ artifact }) => {
    artifact.name = 'npm-darwin-arm64'
  })
  reject(({ artifact }) => {
    artifact.workflow_run.head_repository_id = 2
  })
  reject(({ artifact }) => {
    artifact.workflow_run.repository_id = 2
  })
  reject(({ run }) => {
    run.repository.full_name = 'someone/else'
  })
  reject(({ run }) => {
    run.id = 101
  })
  reject(({ run }) => {
    run.path = '.github/workflows/npm.yml'
  })
  reject(({ run }) => {
    run.status = 'in_progress'
  })
  reject(({ run }) => {
    run.head_sha = 'b'.repeat(40)
  })
  reject(({ run }) => {
    run.head_branch = 'v0.1.0-alpha.4'
  })
  reject(({ run, artifact }) => {
    run.head_branch = artifact.workflow_run.head_branch = 'main'
  })
  reject(({ run, artifact }) => {
    run.head_sha = artifact.workflow_run.head_sha = 'invalid-sha'
  })
  reject(({ run, artifact }) => {
    run.id = artifact.workflow_run.id = -1
  })
  reject(({ jobs }) => {
    jobs[0].jobs[0].conclusion = 'failure'
  })
  reject(({ jobs }) => {
    jobs[1].jobs = []
  })
})
