import assert from 'node:assert/strict'
import { createRequire } from 'node:module'

// This launcher must run directly on every supported publishing Node runtime.
const [npmCli] = process.argv.splice(2, 1)
assert.ok(npmCli, 'expected the npm JavaScript CLI path')
const requireNpm = createRequire(npmCli)
if (process.argv[2] === 'dist-tag') {
  const DistTag = requireNpm('../lib/commands/dist-tag.js')
  const fetchTags = DistTag.prototype.fetchTags
  assert.equal(typeof fetchTags, 'function', 'unsupported npm dist-tag command')
  DistTag.prototype.fetchTags = function (spec, options) {
    // Some registry routes reject npm's lowercase slash escape. Keep npm's
    // OIDC/OTP flow and request logic, changing only equivalent URI encoding.
    spec.escapedName = spec.escapedName.replace(/%2f/gi, '%2F')
    return fetchTags.call(this, spec, options)
  }
}
requireNpm(npmCli)
