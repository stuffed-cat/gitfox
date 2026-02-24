/**
 * Post-install script for GitFox WebIDE
 * 
 * Downloads and extracts openvscode-server static assets
 * which provide the VS Code Web experience.
 */

import { execSync } from 'child_process'
import { existsSync, mkdirSync, createWriteStream, readFileSync, writeFileSync, readdirSync, statSync } from 'fs'
import { pipeline } from 'stream/promises'
import { createGunzip } from 'zlib'
import { extract } from 'tar'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'
import vm from 'vm'

const __dirname = dirname(fileURLToPath(import.meta.url))
const STATIC_DIR = join(__dirname, '..', 'static', 'vscode')
const EXTENSIONS_DIR = join(STATIC_DIR, 'extensions')
const DIST_VSCODE_DIR = join(__dirname, '..', 'dist', 'vscode')
const DIST_DIR = join(__dirname, '..', 'dist')

// openvscode-server version to use
const OPENVSCODE_VERSION = 'v1.109.5'
const RELEASE_URL = `https://github.com/gitpod-io/openvscode-server/releases/download/openvscode-server-${OPENVSCODE_VERSION}/openvscode-server-${OPENVSCODE_VERSION}-linux-x64.tar.gz`

// VS Code Language Packs
const LANGUAGE_PACKS = [
  {
    id: 'ms-ceintl.vscode-language-pack-zh-hans'
  }
]

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

function patchQualityAndCommit() {
  const targets = [STATIC_DIR, DIST_VSCODE_DIR, DIST_DIR].filter((p) => existsSync(p))
  if (targets.length === 0) {
    console.log('No VS Code assets found to patch')
    return
  }

  const replacers = [
    {
      description: 'quality/commit tuple',
      regex: /quality:"[^"]+",version:"([^"]+)",commit:"[^"]+"/g,
      replacement: 'quality:void 0,version:"$1",commit:void 0'
    },
    {
      description: 'productConfiguration.commit literal',
      regex: /productConfiguration\.commit="[^"]+"/g,
      replacement: 'productConfiguration.commit=void 0'
    },
    {
      description: 'productConfiguration.quality literal',
      regex: /productConfiguration\.quality="[^"]+"/g,
      replacement: 'productConfiguration.quality=void 0'
    },
    {
      description: 'CLI commit variable',
      regex: /COMMIT="[a-f0-9]+"/g,
      replacement: 'COMMIT=""'
    },
    {
      description: 'VSCODE_DEV userDataPath override',
      regex: /process\.env\.VSCODE_DEV&&\(([a-z])="code-oss-dev"\)/g,
      replacement: '!1&&($1="code-oss-dev")'
    },
    {
      description: 'VSCODE_DEV product modification',
      regex: /Us\.VSCODE_DEV&&Object\.assign\(As,\{[^}]+\}\)/g,
      replacement: '!1&&Object.assign(As,{})'
    },
    {
      description: 'VSCODE_DEV environment check with assignment',
      regex: /process\.env\.VSCODE_DEV\|\|([a-z])===?"pseudo"/g,
      replacement: '$1==="pseudo"'
    },
    {
      description: 'code-oss-dev string literal',
      regex: /"code-oss-dev"/g,
      replacement: '""'
    },
    {
      description: 'oss-dev string literal',
      regex: /"oss-dev"/g,
      replacement: '""'
    },
    {
      description: 'Force expectsResolverExtension to return false',
      regex: /get expectsResolverExtension\(\)\{return!!this\.options\.remoteAuthority\?\.includes\("\+"\)&&!this\.options\.webSocketFactory\}/g,
      replacement: 'get expectsResolverExtension(){return!1}'
    },
    {
      description: 'Disable remoteAuthority resolution in workspace trust',
      regex: /this\.I\.remoteAuthority&&this\.F\.resolveAuthority\(this\.I\.remoteAuthority\)\.then\(async [^}]+\}\)\.finally\([^}]+\}\)/g,
      replacement: 'Promise.resolve().finally(()=>{this.h()})'
    },
    {
      description: 'Do not unconditionally override remoteAuthority with window.location.host',
      regex: /\{\.\.\.t,remoteAuthority:window\.location\.host\}/g,
      replacement: '{...t,remoteAuthority:t.remoteAuthority}'
    }
  ]

  let patchedFiles = 0
  let patchedReplacements = 0

  function walk(dir) {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry)
      const stat = statSync(full)
      if (stat.isDirectory()) {
        // Skip node_modules to keep traversal fast
        if (entry === 'node_modules') continue
        walk(full)
        continue
      }

      const isJs = full.endsWith('.js')
      const isCli = full.endsWith('openvscode-server')
      if (!isJs && !isCli) continue

      let content = readFileSync(full, 'utf-8')
      let changed = false
      for (const { regex, replacement } of replacers) {
        const next = content.replace(regex, replacement)
        if (next !== content) {
          content = next
          changed = true
          patchedReplacements += 1
        }
      }

      if (changed) {
        writeFileSync(full, content, 'utf-8')
        patchedFiles += 1
      }
    }
  }

  // Patch JS/binary files
  for (const dir of targets) {
    console.log(`Patching VS Code assets in ${dir}...`)
    walk(dir)
  }

  // Patch product.json files
  for (const dir of targets) {
    const productJsonPath = join(dir, 'product.json')
    if (!existsSync(productJsonPath)) continue
    
    try {
      const productJson = JSON.parse(readFileSync(productJsonPath, 'utf-8'))
      let changed = false
      
      if (productJson.quality) {
        productJson.quality = undefined
        changed = true
      }
      if (productJson.commit) {
        productJson.commit = undefined
        changed = true
      }
      
      if (changed) {
        writeFileSync(productJsonPath, JSON.stringify(productJson, null, 2), 'utf-8')
        patchedFiles++
        console.log(`Patched product.json: ${productJsonPath}`)
      }
    } catch (err) {
      console.error(`Failed to patch product.json at ${productJsonPath}:`, err)
    }
  }

  console.log(`Patched ${patchedFiles} files (${patchedReplacements} replacement operations).`)
}

async function downloadLanguagePacks() {
  console.log('\nDownloading language packs...')
  
  for (const pack of LANGUAGE_PACKS) {
    const packDir = join(EXTENSIONS_DIR, pack.id)
    
    if (existsSync(packDir)) {
      console.log(`Language pack ${pack.id} already exists, skipping`)
      continue
    }
    
    console.log(`Downloading ${pack.id}...`)
    
    try {
      // Fetch download URL from Open VSX
      const apiUrl = `https://open-vsx.org/api/${pack.id.replace('.', '/')}/latest`
      const response = await fetch(apiUrl)
      const data = await response.json()
      const downloadUrl = data.files.download
      
      if (!downloadUrl) {
        throw new Error('Download URL not found in API response')
      }
      
      mkdirSync(packDir, { recursive: true })
      
      // Download and extract vsix
      const vsixPath = `/tmp/${pack.id}.vsix`
      const cmd = `curl -L "${downloadUrl}" -o "${vsixPath}" && unzip -q "${vsixPath}" -d "${packDir}" && rm "${vsixPath}"`
      execSync(cmd, { stdio: 'inherit' })
      
      console.log(`✓ Downloaded ${pack.id}`)
    } catch (error) {
      console.error(`Failed to download language pack ${pack.id}:`, error.message)
      console.log('Continuing without this language pack...')
    }
  }
}

function patchVSCodePackageJson() {
  console.log('\nPatching VS Code package.json and creating module alias...')
  
  const packageJsonPath = join(STATIC_DIR, 'package.json')
  if (!existsSync(packageJsonPath)) {
    console.log('VS Code package.json not found, skipping patch')
    return
  }
  
  try {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'))
    
    // Add alias for vscode-regexp-languagedetection -> vscode-regexpp in overrides
    if (!packageJson.overrides) {
      packageJson.overrides = {}
    }
    
    packageJson.overrides['vscode-regexp-languagedetection'] = 'npm:vscode-regexpp@^1'
    
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n', 'utf-8')
    console.log('✓ Added vscode-regexp-languagedetection alias in overrides')
    
    // Create module directory for vscode-regexp-languagedetection
    const nodeModulesDir = join(STATIC_DIR, 'node_modules')
    const regexppDir = join(nodeModulesDir, 'vscode-regexpp')
    const languageDetectionDir = join(nodeModulesDir, 'vscode-regexp-languagedetection')
    
    if (existsSync(regexppDir)) {
      // Remove existing dir if exists
      if (existsSync(languageDetectionDir)) {
        execSync(`rm -rf "${languageDetectionDir}"`, { stdio: 'inherit' })
      }
      
      // Create directory structure
      mkdirSync(join(languageDetectionDir, 'dist'), { recursive: true })
      
      // Create symlinks for files
      const files = ['index.js', 'index.mjs', 'package.json', 'LICENSE']
      for (const file of files) {
        const src = join(regexppDir, file)
        const dest = join(languageDetectionDir, 'dist', file)
        if (existsSync(src)) {
          execSync(`ln -s "${src}" "${dest}"`, { stdio: 'inherit' })
        }
      }
      
      console.log('✓ Created vscode-regexp-languagedetection module structure')
    } else {
      console.log('⚠ vscode-regexpp not found, cannot create alias')
    }
  } catch (error) {
    console.error('Failed to patch VS Code:', error.message)
  }
}

async function generateNLSFiles() {
  console.log('\nGenerating NLS translation files...')
  
  // First, load the English messages as the base
  const enFile = join(STATIC_DIR, 'out', 'nls.messages.js')
  if (!existsSync(enFile)) {
    console.error('English NLS file not found, cannot generate translations')
    return
  }
  
  // Load the English messages using VM sandbox
  const enContent = readFileSync(enFile, 'utf-8')
  const sandbox = { globalThis: { _VSCODE_NLS_MESSAGES: null } }
  vm.createContext(sandbox)
  vm.runInContext(enContent, sandbox)
  const enMessages = sandbox.globalThis._VSCODE_NLS_MESSAGES
  
  if (!enMessages || !Array.isArray(enMessages)) {
    console.error('Could not parse English NLS file')
    return
  }
  
  console.log(`Base English messages: ${enMessages.length}`)
  
  for (const pack of LANGUAGE_PACKS) {
    const packDir = join(EXTENSIONS_DIR, pack.id, 'extension')
    const translationFile = join(packDir, 'translations', 'main.i18n.json')
    
    if (!existsSync(translationFile)) {
      console.log(`Translation file not found for ${pack.id}, skipping`)
      continue
    }
    
    try {
      const translations = JSON.parse(readFileSync(translationFile, 'utf-8'))
      
      // Create translated messages array starting with English as fallback
      const translatedMessages = [...enMessages]
      
      // Build index map from translation file
      const indexMap = {}
      function extractWithIndex(obj, prefix = '') {
        for (const key in obj) {
          const value = obj[key]
          const fullKey = prefix ? `${prefix}/${key}` : key
          if (typeof value === 'string') {
            indexMap[fullKey] = value
          } else if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
            extractWithIndex(value, fullKey)
          }
        }
      }
      
      if (translations.contents) {
        extractWithIndex(translations.contents)
      }
      
      // Read the nls.keys.json to map translations to indices
      const keysFile = join(STATIC_DIR, 'out', 'nls.keys.json')
      if (existsSync(keysFile)) {
        const keysData = JSON.parse(readFileSync(keysFile, 'utf-8'))
        
        // keysData is array of [modulePath, [key1, key2, ...]]
        let messageIndex = 0
        for (const [modulePath, keys] of keysData) {
          for (const key of keys) {
            const fullKey = `${modulePath}/${key}`
            if (indexMap[fullKey]) {
              translatedMessages[messageIndex] = indexMap[fullKey]
            }
            messageIndex++
          }
        }
      }
      
      // Generate the NLS JavaScript file
      const languageId = pack.id.includes('zh-hans') ? 'zh-cn' : 'en'
      const nlsFile = join(STATIC_DIR, 'out', `nls.messages.${languageId}.js`)
      const nlsContent = `globalThis._VSCODE_NLS_MESSAGES=${JSON.stringify(translatedMessages)};`
      
      writeFileSync(nlsFile, nlsContent, 'utf-8')
      console.log(`✓ Generated ${nlsFile} (${translatedMessages.length} messages, ${Object.keys(indexMap).length} translated)`)
    } catch (error) {
      console.error(`Failed to generate NLS file for ${pack.id}:`, error.message)
    }
  }
}

// Only run if called directly
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const patchOnly = process.argv.includes('--patch-only')

  if (patchOnly) {
    patchVSCodePackageJson()
    patchQualityAndCommit()
  } else {
    downloadAndExtract()
      .then(() => patchVSCodePackageJson())
      .then(() => downloadLanguagePacks())
      .then(() => generateNLSFiles())
      .then(() => patchQualityAndCommit())
      .catch(console.error)
  }
}

// export { downloadAndExtract }
