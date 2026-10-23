import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export const repositoryRoot: string = fileURLToPath(
  new URL('../../', import.meta.url),
)

export const publicNpmPackage: string = '@gdfmt/gdfmt'
export const wasmNpmPackage: string = '@gdfmt/wasm'

/**
 * Frozen legacy releases predate the WASM host; recovery uses their own source.
 */
export function hasWasmHost(root: string = repositoryRoot): boolean {
  return existsSync(resolve(root, 'crates/gdfmt_wasm/Cargo.toml'))
}

/**
 * Dependency order for the public Rust packages. Host adapters stay private.
 */
export const publicCrates: readonly string[] = [
  'gdfmt_text',
  'gdfmt_syntax',
  'gdfmt_parser',
  'gdfmt_formatter',
  'gdfmt',
  'gdfmt_config',
  'gdfmt_workspace',
  'gdfmt_cli',
]
export const privateCrates: readonly string[] = [
  'gdfmt_napi',
  'gdfmt_wasm',
  'xtask',
]

export function readObject(value: unknown): Record<string, unknown> {
  assert.ok(
    value && typeof value === 'object' && !Array.isArray(value),
    'expected an object',
  )
  return Object.fromEntries(Object.entries(value))
}

export function readJson(path: string): Record<string, unknown> {
  return readObject(JSON.parse(readFileSync(path, 'utf8')))
}

/**
 * npm drops build metadata, so releases accept SemVer without a +build suffix.
 */
export function validateVersion(version: string): void {
  assert.ok(
    version.length <= 256,
    'version exceeds the npm SemVer length limit',
  )
  assert.match(
    version,
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/,
    'expected a SemVer version without build metadata',
  )
  for (const part of version.split('-')[0].split('.')) {
    assert.ok(
      BigInt(part) <= BigInt(Number.MAX_SAFE_INTEGER),
      'version exceeds the npm SemVer numeric limit',
    )
  }
  for (const part of version.split('-').slice(1).join('-').split('.')) {
    assert.ok(
      !/^0\d+$/.test(part),
      'numeric prerelease identifiers cannot have leading zeros',
    )
  }
}

export function releaseChannel(version: string): 'next' | 'latest' {
  validateVersion(version)
  return version.includes('-') ? 'next' : 'latest'
}

/**
 * SemVer precedence, including numeric prerelease identifiers.
 */
export function compareVersions(left: string, right: string): number {
  validateVersion(left)
  validateVersion(right)
  const parse = (value: string): { core: string[]; prerelease: string[] } => {
    const separator = value.indexOf('-')
    return {
      core: (separator < 0 ? value : value.slice(0, separator)).split('.'),
      prerelease: separator < 0 ? [] : value.slice(separator + 1).split('.'),
    }
  }
  const a = parse(left)
  const b = parse(right)
  for (let index = 0; index < 3; index++) {
    if (BigInt(a.core[index]) !== BigInt(b.core[index]))
      return BigInt(a.core[index]) > BigInt(b.core[index]) ? 1 : -1
  }
  if (!a.prerelease.length || !b.prerelease.length)
    return Number(!a.prerelease.length) - Number(!b.prerelease.length)
  for (
    let index = 0;
    index < Math.max(a.prerelease.length, b.prerelease.length);
    index++
  ) {
    const first = a.prerelease[index]
    const second = b.prerelease[index]
    if (first === second) continue
    if (first === undefined || second === undefined)
      return first === undefined ? -1 : 1
    const firstNumeric = /^\d+$/.test(first)
    const secondNumeric = /^\d+$/.test(second)
    if (firstNumeric && secondNumeric)
      return BigInt(first) > BigInt(second) ? 1 : -1
    if (firstNumeric !== secondNumeric) return firstNumeric ? -1 : 1
    return first > second ? 1 : -1
  }
  return 0
}

export function gitRevision(root: string = repositoryRoot): string {
  return execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: root,
    encoding: 'utf8',
  }).trim()
}

export function isCleanReleaseSource(root: string = repositoryRoot): boolean {
  return (
    execFileSync('git', ['status', '--porcelain', '--untracked-files=normal'], {
      cwd: root,
      encoding: 'utf8',
    }).trim() === ''
  )
}

export function assertCleanReleaseSource(root: string = repositoryRoot): void {
  assert.ok(
    isCleanReleaseSource(root),
    'publication requires a clean checkout of the frozen release commit, including staged and untracked files',
  )
}

export function workspaceVersion(source: string): string {
  const lines = source.split(/\r?\n/)
  const start = lines.indexOf('[workspace.package]')
  assert.ok(start >= 0, 'missing workspace.package section')
  const section = lines
    .slice(start + 1)
    .join('\n')
    .split(/^\[/m)[0]
  const version = section.match(/^version = "([^"]+)"$/m)?.[1]
  assert.ok(version, 'missing workspace.package.version')
  validateVersion(version)
  return version
}

/**
 * Checks every maintained version and the publish allowlist before any registry write.
 */
export function checkRelease(
  root: string = repositoryRoot,
  tag?: string,
): string {
  const cargo = readFileSync(resolve(root, 'Cargo.toml'), 'utf8')
  const version = workspaceVersion(cargo)
  if (tag !== undefined)
    assert.equal(
      tag,
      `v${version}`,
      'release tag must match the package version',
    )
  const manifest = readJson(resolve(root, 'packages/gdfmt/package.json'))
  assert.equal(manifest.name, publicNpmPackage)
  assert.equal(manifest.version, version, 'Cargo/npm version mismatch')
  assert.notEqual(
    manifest.private,
    true,
    'the public npm facade cannot be private',
  )
  assert.equal(readObject(manifest.publishConfig).access, 'public')
  assert.equal(
    readObject(manifest.repository).url,
    'git+https://github.com/ntnyq/gdfmt.git',
    'npm repository must match the Trusted Publisher repository',
  )
  if (hasWasmHost(root)) {
    const wasm = readJson(resolve(root, 'packages/wasm/package.json'))
    assert.equal(wasm.name, wasmNpmPackage)
    assert.equal(wasm.version, version, 'Cargo/WASM version mismatch')
    assert.notEqual(wasm.private, true)
    assert.equal(readObject(wasm.publishConfig).access, 'public')
    assert.equal(
      readObject(wasm.repository).url,
      'git+https://github.com/ntnyq/gdfmt.git',
    )
    const extension = readJson(resolve(root, 'editors/vscode/package.json'))
    assert.equal(
      extension.gdfmtVersion,
      version,
      'VS Code/core version mismatch',
    )
    assert.ok(
      typeof extension.version === 'string' &&
        /^\d+\.\d+\.\d+$/.test(extension.version),
      'VS Code requires a numeric Marketplace version',
    )
    validateVersion(extension.version)
  }
  const targets: unknown = JSON.parse(
    readFileSync(resolve(root, 'packages/gdfmt/src/targets.json'), 'utf8'),
  )
  assert.ok(Array.isArray(targets))
  const expected = Object.fromEntries(
    targets.map(target => {
      const id = readObject(target).id
      assert.equal(typeof id, 'string')
      return [`@gdfmt/${id}`, version]
    }),
  )
  assert.deepEqual(
    manifest.optionalDependencies,
    expected,
    'platform packages must match the root version and target inventory',
  )
  const metadata = readObject(
    JSON.parse(
      execFileSync(
        'cargo',
        ['metadata', '--no-deps', '--locked', '--format-version', '1'],
        { cwd: root, encoding: 'utf8' },
      ),
    ),
  )
  assert.ok(Array.isArray(metadata.packages))
  const packages = metadata.packages.map(readObject)
  assert.deepEqual(
    packages.map(item => item.name).sort(),
    [
      ...publicCrates,
      ...privateCrates.filter(
        name => name !== 'gdfmt_wasm' || hasWasmHost(root),
      ),
    ].sort(),
    'update the release allowlist for workspace changes',
  )
  const lock = readFileSync(resolve(root, 'Cargo.lock'), 'utf8')
    .split('[[package]]')
    .slice(1)
  for (const item of packages) {
    assert.ok(typeof item.name === 'string')
    const name = item.name
    assert.equal(item.version, version, `${item.name} version mismatch`)
    const locked = lock.find(
      block =>
        block.match(/^name = "([^"]+)"$/m)?.[1] === item.name &&
        !/^source = /m.test(block),
    )
    assert.equal(
      locked?.match(/^version = "([^"]+)"$/m)?.[1],
      version,
      `${item.name} Cargo.lock version mismatch`,
    )
    if (!publicCrates.includes(name)) {
      assert.deepEqual(item.publish, [], `${item.name} must remain unpublished`)
      continue
    }
    assert.deepEqual(
      item.publish,
      ['crates-io'],
      `${item.name} must only publish to crates.io`,
    )
    assert.equal(typeof item.description, 'string')
    assert.equal(
      item.repository,
      'https://github.com/ntnyq/gdfmt',
      `${name} repository must match the publishing repository`,
    )
    assert.ok(
      item.description && item.readme && item.license === 'MIT',
      `${item.name} needs public metadata`,
    )
    assert.equal(
      readFileSync(resolve(root, 'crates', name, 'LICENSE'), 'utf8'),
      readFileSync(resolve(root, 'LICENSE'), 'utf8'),
      `${name} license must match the workspace`,
    )
    assert.ok(Array.isArray(item.dependencies))
    for (const dependency of item.dependencies.map(readObject)) {
      if (
        typeof dependency.name !== 'string' ||
        !publicCrates.includes(dependency.name)
      )
        continue
      assert.equal(
        dependency.req,
        `=${version}`,
        `${item.name} → ${dependency.name} must use the exact workspace version`,
      )
      if (dependency.kind !== 'dev')
        assert.ok(
          publicCrates.indexOf(dependency.name) < publicCrates.indexOf(name),
          'public crates must be in dependency order',
        )
    }
  }
  assert.equal(
    readFileSync(
      resolve(root, 'crates/gdfmt_parser/LICENSE-THIRD-PARTY'),
      'utf8',
    ),
    readFileSync(resolve(root, 'LICENSE-THIRD-PARTY'), 'utf8'),
    'parser attribution must match the workspace',
  )
  return version
}
