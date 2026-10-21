#!/usr/bin/env node
try {
  const { binding } = require('./binding.cjs')
  process.exitCode = binding.runCli(process.argv.slice(2))
} catch (error) {
  console.error(error.message)
  if (error.cause) console.error(error.cause.message)
  process.exitCode = 2
}
