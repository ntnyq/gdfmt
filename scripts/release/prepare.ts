import assert from 'node:assert/strict'
import { join, resolve } from 'node:path'
import { prepareRelease } from './artifacts.ts'
import { prepareCrates } from './crate-artifacts.ts'
import { prepareEditors } from './editor-artifacts.ts'

const args = process.argv.slice(2)
assert.ok(
  args.length <= 2 && args.every(arg => !arg.startsWith('--')),
  'usage: pnpm release:prepare [MATRIX_DIRECTORY] [BUNDLE_DIRECTORY]',
)
const destination = resolve(args[1] || 'target/release')
await prepareRelease(
  resolve(args[0] || 'target/npm-ci'),
  join(destination, 'npm'),
)
await prepareCrates(resolve('target/package'), join(destination, 'crates'))

await prepareEditors(resolve('target/editor-ci'), join(destination, 'editors'))
