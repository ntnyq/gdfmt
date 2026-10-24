import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  checkRelease,
  compareVersions,
  privateCrates,
  publicCrates,
  readJson,
  repositoryRoot,
  validateVersion,
  workspaceVersion,
} from './project.ts'

/**
 * Plans a version change without creating commits, tags or registry requests.
 */
export function planVersionUpdate(
  root: string,
  version: string,
): Map<string, string> {
  validateVersion(version)
  const cargoPath = resolve(root, 'Cargo.toml')
  const cargo = readFileSync(cargoPath, 'utf8')
  const current = workspaceVersion(cargo)
  assert.ok(
    compareVersions(version, current) > 0,
    'new version must be greater than the current version',
  )
  const files = new Map<string, string>()
  let dependencyCount = 0
  const updatedCargo = cargo
    .replace(
      /^(\[workspace\.package\]\r?\n[\s\S]*?^version = ")[^"\r\n]+("(?=\r?$))/m,
      (_line: string, prefix: string, suffix: string) =>
        `${prefix}${version}${suffix}`,
    )
    .replace(
      /^(gdfmt\w*) = \{ path = "([^"]+)", version = "([^"]+)" \}$/gm,
      (line: string, name: string, path: string, requirement: string) => {
        assert.ok(
          publicCrates.includes(name),
          `unexpected internal dependency: ${name}`,
        )
        assert.equal(requirement, `=${current}`)
        dependencyCount++
        return `${name} = { path = "${path}", version = "=${version}" }`
      },
    )
  assert.equal(
    dependencyCount,
    publicCrates.length - 1,
    'expected all workspace dependency versions',
  )
  assert.equal(
    workspaceVersion(updatedCargo),
    version,
    'workspace version was not updated',
  )
  files.set(cargoPath, updatedCargo)
  const lockPath = resolve(root, 'Cargo.lock')
  let packageCount = 0
  const updatedLock = readFileSync(lockPath, 'utf8')
    .split('[[package]]')
    .map(block => {
      const name = block.match(/^name = "([^"]+)"$/m)?.[1]
      if (
        !name ||
        ![...publicCrates, ...privateCrates].includes(name) ||
        /^source = /m.test(block)
      )
        return block
      assert.ok(
        block.includes(`version = "${current}"`),
        `unexpected locked version: ${name}`,
      )
      packageCount++
      return block.replace(`version = "${current}"`, `version = "${version}"`)
    })
    .join('[[package]]')
  assert.equal(packageCount, publicCrates.length + privateCrates.length)
  files.set(lockPath, updatedLock)
  const npmPath = resolve(root, 'packages/gdfmt/package.json')
  const manifest = readJson(npmPath)
  assert.equal(manifest.version, current)
  assert.ok(
    manifest.optionalDependencies &&
      typeof manifest.optionalDependencies === 'object',
  )
  manifest.version = version
  manifest.optionalDependencies = Object.fromEntries(
    Object.entries(manifest.optionalDependencies).map(([name, value]) => {
      assert.equal(value, current, `unexpected platform version: ${name}`)
      return [name, version]
    }),
  )
  files.set(npmPath, `${JSON.stringify(manifest, null, 2)}\n`)
  for (const [relative, field] of [
    ['packages/wasm/package.json', 'version'],
    ['editors/vscode/package.json', 'gdfmtVersion'],
  ]) {
    const path = resolve(root, relative)
    const manifest = readJson(path)
    assert.equal(manifest[field], current)
    manifest[field] = version
    files.set(path, `${JSON.stringify(manifest, null, 2)}\n`)
  }
  // The pnpm lockfile does not store workspace package versions. Platform packages
  // are generated artifacts, not workspace members or installed build dependencies.
  const pnpmLock = readFileSync(resolve(root, 'pnpm-lock.yaml'), 'utf8')
  assert.ok(
    !pnpmLock.includes(`specifier: ${current}`),
    'platform dependencies are now locked: update the version script before releasing',
  )
  return files
}

export function updateVersion(
  root: string,
  version: string,
  dryRun: boolean = false,
): void {
  checkRelease(root)
  const files = planVersionUpdate(root, version)
  if (!dryRun) {
    const originals = new Map(
      [...files.keys()].map(path => [path, readFileSync(path, 'utf8')]),
    )
    try {
      for (const [path, text] of files) writeFileSync(path, text)
      checkRelease(root)
    } catch (error) {
      for (const [path, text] of originals) writeFileSync(path, text)
      throw error
    }
  }
  console.log(
    `${dryRun ? 'Would update' : 'Updated'} Cargo, npm and VS Code core versions to ${version}. No commit, tag or publication created.`,
  )
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [version, ...flags] = process.argv.slice(2)
  assert.ok(
    version && flags.every(flag => flag === '--dry-run') && flags.length <= 1,
    'usage: pnpm release:version VERSION [--dry-run]',
  )
  updateVersion(repositoryRoot, version, flags.includes('--dry-run'))
}
