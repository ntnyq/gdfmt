import { appendFileSync } from 'node:fs'
import { checkRelease, releaseChannel } from './project.ts'

const args = process.argv.slice(2)
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--tag'))
  throw new Error('usage: pnpm release:check [--tag vVERSION]')
const version = checkRelease(undefined, args[1])
const channel = releaseChannel(version)
if (process.env.GITHUB_OUTPUT)
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    `version=${version}\nchannel=${channel}\n`,
  )
console.log(
  `Release configuration valid: ${version} (${channel}). No publication performed.`,
)
