/**
 * Identifies release stages and retry activity without logging credentials.
 */
export function logReleaseProgress(message: string): void {
  console.log(`[release ${new Date().toISOString()}] ${message}`)
}
