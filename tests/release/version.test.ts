import assert from 'node:assert/strict'
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import {
  checkRelease,
  compareVersions,
  privateCrates,
  publicCrates,
  readObject,
  releaseChannel,
  repositoryRoot,
  validateVersion,
} from '../../scripts/release/project.ts'
import {
  planVersionUpdate,
  updateVersion,
} from '../../scripts/release/version.ts'

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), 'gdfmt-version-'))
  for (const file of [
    'Cargo.toml',
    'Cargo.lock',
    'pnpm-lock.yaml',
    'LICENSE',
    'LICENSE-THIRD-PARTY',
    'crates/gdfmt_parser/LICENSE-THIRD-PARTY',
    'packages/gdfmt/package.json',
    'packages/wasm/package.json',
    'editors/vscode/package.json',
    'packages/gdfmt/src/targets.json',
  ]) {
    const destination = join(root, file)
    mkdirSync(join(destination, '..'), { recursive: true })
    copyFileSync(join(repositoryRoot, file), destination)
  }
  for (const name of [...publicCrates, ...privateCrates]) {
    const directory = name === 'xtask' ? 'xtask' : `crates/${name}`
    mkdirSync(join(root, directory, 'src'), { recursive: true })
    copyFileSync(
      join(repositoryRoot, directory, 'Cargo.toml'),
      join(root, directory, 'Cargo.toml'),
    )
    if (publicCrates.includes(name))
      copyFileSync(
        join(repositoryRoot, 'LICENSE'),
        join(root, directory, 'LICENSE'),
      )
    writeFileSync(join(root, directory, 'src/lib.rs'), '')
    writeFileSync(join(root, directory, 'src/main.rs'), 'fn main() {}\n')
  }
  return root
}

test('version update synchronizes every public and private Rust package, dependency and npm platform', () => {
  const root = fixture()
  try {
    const before = checkRelease(root)
    const currentMajor = before.split('.')[0]
    const next = `${BigInt(currentMajor) + 1n}.0.0-alpha.0`
    const rootManifest = readFileSync(
      join(repositoryRoot, 'package.json'),
      'utf8',
    )
    updateVersion(root, next)
    assert.equal(checkRelease(root, `v${next}`), next)
    assert.throws(() => checkRelease(root, `v${before}`), /release tag/)
    assert.equal(
      readFileSync(join(repositoryRoot, 'package.json'), 'utf8'),
      rootManifest,
    )
    assert.equal(
      readFileSync(join(root, 'pnpm-lock.yaml'), 'utf8'),
      readFileSync(join(repositoryRoot, 'pnpm-lock.yaml'), 'utf8'),
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('dry-run and invalid versions never modify release files', () => {
  const root = fixture()
  try {
    const current = checkRelease(root)
    const path = join(root, 'Cargo.toml')
    const original = readFileSync(path, 'utf8')
    updateVersion(root, '999.0.0', true)
    assert.equal(readFileSync(path, 'utf8'), original)
    for (const version of [
      'v1.0.0',
      '1.0',
      '01.0.0',
      '1.0.0-alpha.01',
      '1.0.0+build',
      '1.0.0;echo x',
      current,
      '0.0.0',
    ])
      assert.throws(() => planVersionUpdate(root, version))
    assert.equal(readFileSync(path, 'utf8'), original)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('release checks reject inconsistent optional dependencies and unsafe publish configuration', () => {
  const root = fixture()
  try {
    const path = join(root, 'packages/gdfmt/package.json')
    const original = readFileSync(path, 'utf8')
    const manifest = readObject(JSON.parse(original))
    manifest.optionalDependencies = {
      ...readObject(manifest.optionalDependencies),
      '@gdfmt/darwin-arm64': '9.9.9',
    }
    writeFileSync(path, JSON.stringify(manifest))
    assert.throws(() => checkRelease(root), /platform packages/)
    assert.throws(() => updateVersion(root, '999.0.0'), /platform packages/)
    writeFileSync(path, original)
    writeFileSync(
      path,
      JSON.stringify({
        ...readObject(JSON.parse(original)),
        repository: {
          type: 'git',
          url: 'git+https://github.com/example/wrong.git',
        },
      }),
    )
    assert.throws(() => checkRelease(root), /Trusted Publisher repository/)
    writeFileSync(path, original)
    const cargo = join(root, 'crates/gdfmt_text/Cargo.toml')
    writeFileSync(
      cargo,
      readFileSync(cargo, 'utf8').replace(
        'publish = ["crates-io"]',
        'publish = false',
      ),
    )
    assert.throws(() => checkRelease(root), /must only publish/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SemVer channels and precedence handle prereleases and numeric identifiers', () => {
  const versions = [
    '1.0.0-alpha',
    '1.0.0-alpha.1',
    '1.0.0-alpha.2',
    '1.0.0-alpha.10',
    '1.0.0-beta',
    '1.0.0-rc.1',
    '1.0.0',
    '1.0.1',
    '2.0.0',
  ]
  for (let index = 1; index < versions.length; index++) {
    assert.equal(compareVersions(versions[index - 1], versions[index]), -1)
    assert.equal(compareVersions(versions[index], versions[index - 1]), 1)
  }
  assert.equal(compareVersions('1.0.0', '1.0.0'), 0)
  assert.equal(releaseChannel('1.0.0-rc.1'), 'next')
  assert.equal(releaseChannel('1.0.0'), 'latest')
  assert.throws(() => validateVersion('1.0.0+build'))
  assert.throws(() => validateVersion('9007199254740992.0.0'))
})
