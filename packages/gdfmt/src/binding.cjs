const { createRequire } = require('node:module')
const { platform, arch, report } = require('node:process')
const { isAbsolute } = require('node:path')
const targets = require('./targets.json')
const { version } = require('../package.json')

let libc
if (platform === 'linux') {
  // No shell probes or compiler fallback during installation/loading.
  const header = report.getReport().header
  libc = header.glibcVersionRuntime ? 'glibc' : 'musl'
}
const target = targets.find(
  item => item.os === platform && item.cpu === arch && item.libc === libc,
)
if (!target) {
  throw new Error(
    `gdfmt: unsupported native target ${platform}-${arch}${libc ? `-${libc}` : ''}`,
  )
}

const name = `@gdfmt/${target.id}`
let binding
try {
  const metadataPath = require.resolve(`${name}/package.json`)
  const metadata = require(metadataPath)
  if (metadata.name !== name || metadata.version !== version) {
    throw new Error(
      `expected ${name}@${version}, found ${metadata.name}@${metadata.version}`,
    )
  }
  // Resolve from the checked package, avoiding a second independent lookup.
  binding = createRequire(metadataPath)('./gdfmt.node')
  if (
    binding.packageVersion() !== version ||
    binding.nativeTarget() !== target.triple
  ) {
    throw new Error(
      `native version/target mismatch; expected ${version} / ${target.triple}`,
    )
  }
} catch (cause) {
  throw new Error(
    `gdfmt: cannot load ${name}@${version}. Install with optional dependencies enabled for this platform.`,
    { cause },
  )
}

module.exports = { binding, isAbsolute }
