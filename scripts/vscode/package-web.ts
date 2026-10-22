import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createVSIX } from '@vscode/vsce'
import {
  checkRelease,
  gitRevision,
  isCleanReleaseSource,
  readJson,
  repositoryRoot,
} from '../release/project.ts'
import { buildWebExtension } from './build-web.ts'
import { extensionRoot } from './build.ts'
import { stageExtension } from './stage.ts'

export const webStagingRoot: string = resolve(
  repositoryRoot,
  'target/editors/staging/web',
)

export async function packageWebExtension(): Promise<string> {
  const version = checkRelease()
  await buildWebExtension()
  await stageExtension(webStagingRoot, 'web')
  const directory = resolve(repositoryRoot, 'target/editors/web')
  await mkdir(directory, { recursive: true })
  const file = `gdfmt-${version}-web.vsix`
  const path = resolve(directory, file)
  await createVSIX({
    cwd: webStagingRoot,
    packagePath: path,
    target: 'web',
    dependencies: false,
    preRelease: version.includes('-'),
  })
  await writeFile(
    resolve(directory, 'acceptance.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        revision: gitRevision(),
        cleanSource: isCleanReleaseSource(),
        coreVersion: version,
        extensionVersion: readJson(resolve(extensionRoot, 'package.json'))
          .version,
        target: 'web',
        file,
        sha512: createHash('sha512')
          .update(await readFile(path))
          .digest('hex'),
        webExtensionHostPassed: false,
        published: false,
      },
      null,
      2,
    )}\n`,
  )
  return path
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  console.log(await packageWebExtension())
