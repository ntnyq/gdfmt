import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import compatibility from './npm-compat.json' with { type: 'json' }

// This launcher must run directly on every supported publishing Node runtime.
const [npmCli] = process.argv.splice(2, 1)
assert.ok(npmCli, 'expected the npm JavaScript CLI path')
const requireNpm = createRequire(npmCli)
if (process.argv[2] === 'dist-tag') {
  assert.equal(
    requireNpm('../package.json').version,
    compatibility.version,
    `The temporary dist-tag patch only supports npm ${compatibility.version}; validate unpatched OIDC dist-tags before changing npm (see RELEASING.md)`,
  )
  const DistTag = requireNpm('../lib/commands/dist-tag.js')
  const fetchTags = DistTag.prototype.fetchTags
  assert.equal(typeof fetchTags, 'function', 'unsupported npm dist-tag command')
  DistTag.prototype.fetchTags = function (spec, options) {
    // Temporary workaround observed during alpha.5 recovery. Remove after an
    // unpatched, pinned npm passes scoped GET/PUT and GitHub OIDC promotion;
    // see RELEASING.md for the removal checks. Preserve npm's auth handling.
    spec.escapedName = spec.escapedName.replace(/%2f/gi, '%2F')
    return fetchTags.call(this, spec, options)
  }
}
requireNpm(npmCli)
