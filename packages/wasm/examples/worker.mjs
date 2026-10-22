import { createFormatter } from '../dist/index.mjs'

const formatter = await createFormatter()
self.onmessage = ({ data: { id, source, options } }) => {
  try {
    self.postMessage({ id, result: formatter.formatSync(source, options) })
  } catch (error) {
    self.postMessage({ id, error: String(error) })
  }
}
self.postMessage({ ready: true })
