/**
 * Post-install script for GitFox WebIDE
 * 
 * Downloads and extracts openvscode-server static assets
 * which provide the VS Code Web experience.
 */

import { execSync } from 'child_process'
import { existsSync, mkdirSync, createWriteStream } from 'fs'
import { pipeline } from 'stream/promises'
import { createGunzip } from 'zlib'
import { extract } from 'tar'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const STATIC_DIR = join(__dirname, '..', 'static', 'vscode')

// openvscode-server version to use
const OPENVSCODE_VERSION = 'v1.109.5'
const RELEASE_URL = `https://github.com/gitpod-io/openvscode-server/releases/download/openvscode-server-${OPENVSCODE_VERSION}/openvscode-server-${OPENVSCODE_VERSION}-linux-x64.tar.gz`

async function downloadAndExtract() {
  if (existsSync(STATIC_DIR)) {
    console.log('VS Code static assets already exist, skipping download')
    return
  }

  console.log(`Downloading openvscode-server ${OPENVSCODE_VERSION}...`)
  
  mkdirSync(STATIC_DIR, { recursive: true })

  try {
    // Use curl/wget for download (more reliable for large files)
    const curlCmd = `curl -L "${RELEASE_URL}" | tar -xzf - -C "${STATIC_DIR}" --strip-components=1`
    execSync(curlCmd, { stdio: 'inherit' })
    
    console.log('VS Code static assets downloaded successfully')
  } catch (error) {
    console.error('Failed to download VS Code assets:', error)
    console.log('\nYou can manually download from:')
    console.log(RELEASE_URL)
    console.log(`\nAnd extract to: ${STATIC_DIR}`)
    process.exit(1)
  }
}

// Only run if called directly
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  downloadAndExtract().catch(console.error)
}

// export { downloadAndExtract }
