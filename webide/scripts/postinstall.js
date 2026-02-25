/**
 * Post-install script for GitFox WebIDE
 * 
 * Downloads and extracts openvscode-server static assets
 * which provide the VS Code Web experience.
 */

import { execSync } from 'child_process'
import { existsSync, mkdirSync, createWriteStream, readFileSync, writeFileSync, readdirSync, statSync, symlinkSync, lstatSync, unlinkSync } from 'fs'
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

// 需要额外下载完整版本的扩展（openvscode-server 版本缺失 browser 构建）
const ADDITIONAL_WEB_EXTENSIONS = [
  { name: 'emmet' }
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
    },
    {
      // commit 字段被我们清空后 BrowserWorkbenchEnvironmentService.isBuilt 返回 false，
      // 导致静态内置扩展（typescript-basics 等语法高亮）数组女设置为空，语法高亮全部失效。
      // 强制返回 true 修复此问题。
      description: 'Force isBuilt to return true so static builtin extensions (grammars) are loaded',
      regex: /get isBuilt\(\)\{return!!this\.f\.commit\}/g,
      replacement: 'get isBuilt(){return!0}'
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

/**
 * 在 extensions/ 下为 vscode-oniguruma 创建指向 node_modules 的符号链接。
 * Web 模式下 VS Code 从 extensions/vscode-oniguruma/release/onig.wasm 加载 WASM，
 * 而 openvscode-server 将其打包在 node_modules/ 中，需要此链接桥接。
 */
function createOnigurumaSymlink() {
  const dirs = [EXTENSIONS_DIR, join(DIST_VSCODE_DIR, 'extensions')].filter(existsSync)
  for (const extDir of dirs) {
    const linkPath = join(extDir, 'vscode-oniguruma')
    const targetPath = join(extDir, '..', 'node_modules', 'vscode-oniguruma')
    if (!existsSync(targetPath)) {
      console.log(`vscode-oniguruma not found at ${targetPath}, skipping symlink`)
      continue
    }
    // 如果已经存在（文件或链接），跳过
    try {
      lstatSync(linkPath)
      console.log('extensions/vscode-oniguruma already exists, skipping symlink')
      continue
    } catch {
      // 不存在，继续创建
    }
    try {
      symlinkSync('../node_modules/vscode-oniguruma', linkPath)
      console.log(`Created symlink: extensions/vscode-oniguruma -> ../node_modules/vscode-oniguruma`)
    } catch (err) {
      console.error(`Failed to create vscode-oniguruma symlink:`, err.message)
    }
  }
}

/**
 * 为某些只有 node 版本的扩展创建 browser 版本的软链接或副本
 * emmet 等扩展在 package.json 中声明了 browser 字段，但实际只有 node 版本
 */
function createBrowserVersionLinks() {
  const extensionBrowserMappings = [
    {
      name: 'emmet',
      nodeMain: 'dist/node/emmetNodeMain.js',
      browserMain: 'dist/browser/emmetBrowserMain.js'
    },
    {
      name: 'php-language-features',
      nodeMain: 'dist/phpMain.js',
      browserMain: 'dist/browser/phpBrowserMain.js'
    }
  ]

  const dirs = [EXTENSIONS_DIR, join(DIST_VSCODE_DIR, 'extensions')].filter(existsSync)
  let created = 0

  for (const dir of dirs) {
    for (const mapping of extensionBrowserMappings) {
      const extDir = join(dir, mapping.name)
      if (!existsSync(extDir)) continue

      const nodeMainPath = join(extDir, mapping.nodeMain)
      const browserMainPath = join(extDir, mapping.browserMain)

      // 如果 node 版本存在但 browser 版本不存在
      if (existsSync(nodeMainPath) && !existsSync(browserMainPath)) {
        try {
          // 创建 browser 目录
          const browserDir = dirname(browserMainPath)
          mkdirSync(browserDir, { recursive: true })

          // 创建符号链接指向 node 版本
          const relativePath = join('..', '..', mapping.nodeMain)
          symlinkSync(relativePath, browserMainPath)
          
          console.log(`✓ Created browser symlink for ${mapping.name}: ${mapping.browserMain} -> ${mapping.nodeMain}`)
          created++

          // 同时更新 package.json 的 browser 字段
          const pkgPath = join(extDir, 'package.json')
          if (existsSync(pkgPath)) {
            try {
              const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'))
              pkg.browser = './' + mapping.browserMain.replace(/\.js$/, '')
              writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf-8')
              console.log(`  ✓ Updated ${mapping.name}/package.json browser field`)
            } catch (err) {
              console.error(`  ⚠ Failed to update package.json for ${mapping.name}:`, err.message)
            }
          }
        } catch (err) {
          console.error(`Failed to create browser link for ${mapping.name}:`, err.message)
        }
      }
    }
  }

  if (created > 0) {
    console.log(`Created ${created} browser version links for extensions`)
  }
}

/**
 * 为 VS Code 内置扩展的 package.json 添加 extensionKind: ["web"]
 * grammar/language/theme 等纯声明式扩展没有此字段时 VS Code web 会跳过加载。
 */
function patchExtensionsForWeb() {
  const dirs = [EXTENSIONS_DIR, join(DIST_VSCODE_DIR, 'extensions')].filter(existsSync)
  if (dirs.length === 0) {
    console.log('No extensions directory found, skipping web extension patch')
    return
  }

  let patched = 0
  for (const dir of dirs) {
    for (const entry of readdirSync(dir)) {
      const pkgPath = join(dir, entry, 'package.json')
      if (!existsSync(pkgPath)) continue
      try {
        const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'))

        // 已有 extensionKind 的跳过（避免覆盖有意设为 ui-only 的扩展）
        if (pkg.extensionKind) continue

        const hasBrowser = !!pkg.browser
        const hasMain = !!pkg.        main

        // 只有 main 没有 browser → 仅桌面端，跳过
        if (hasMain && !hasBrowser) continue

        // 有 browser 字段时，检查文件是否真的存在（有些构建只保留了 main）
        if (hasBrowser) {
          const browserFile = pkg.browser.replace(/\.(js)?$/, '') + '.js'
          const browserPath = join(dir, entry, browserFile)
          if (!existsSync(browserPath)) continue // browser 文件不存在，不加
        }

        pkg.extensionKind = ['web']
        writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf-8')
        patched++
      } catch {
        // 跳过解析失败的扩展
      }
    }
  }
  console.log(`Patched ${patched} extension package.json files with extensionKind: ["web"]`)
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

/**
 * 下载缺失 browser 版本的完整扩展
 * openvscode-server 某些扩展没有编译 browser 版本，从 Open VSX 下载官方版本
 */
async function downloadMissingWebExtensions() {
  console.log('\nDownloading missing web extension browser builds...')
  
  for (const ext of ADDITIONAL_WEB_EXTENSIONS) {
    const extDir = join(EXTENSIONS_DIR, ext.name)
    const pkgPath = join(extDir, 'package.json')
    
    // 检查 browser 版本是否已存在且有效
    if (existsSync(pkgPath)) {
      try {
        const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'))
        if (pkg.browser) {
          const browserFile = pkg.browser.replace(/\.(js)?$/, '') + '.js'
          const browserPath = join(extDir, browserFile)
          if (existsSync(browserPath)) {
            console.log(`✓ ${ext.name} already has valid browser version`)
            continue
          }
        }
      } catch {}
    }
    
    console.log(`Downloading ${ext.name} from Open VSX...`)
    
    try {
      // 从 Open VSX 获取扩展信息
      const apiUrl = `https://open-vsx.org/api/vscode/${ext.name}/latest`
      console.log(`  Fetching extension info from ${apiUrl}...`)
      
      const response = await fetch(apiUrl)
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`)
      }
      
      const data = await response.json()
      const downloadUrl = data.files?.download
      
      if (!downloadUrl) {
        throw new Error('Download URL not found in API response')
      }
      
      const tmpDir = `/tmp/vscode-ext-${Date.now()}`
      mkdirSync(tmpDir, { recursive: true })
      
      // 下载并解压 VSIX
      const vsixPath = `${tmpDir}/${ext.name}.vsix`
      console.log(`  Downloading from ${downloadUrl}...`)
      const downloadCmd = `curl -L "${downloadUrl}" -o "${vsixPath}" && unzip -q "${vsixPath}" -d "${tmpDir}"`
      execSync(downloadCmd, { stdio: 'pipe' })
      
      // 备份旧版本
      if (existsSync(extDir)) {
        execSync(`mv "${extDir}" "${extDir}.backup"`, { stdio: 'pipe' })
      }
      
      // copyextension 目录
      const extSourceDir = join(tmpDir, 'extension')
      if (existsSync(extSourceDir)) {
        execSync(`cp -r "${extSourceDir}" "${extDir}"`, { stdio: 'inherit' })
                // 修改 package.json，移除 main 字段以强制使用 browser 版本
        const newPkgPath = join(extDir, 'package.json')
        if (existsSync(newPkgPath)) {
          try {
            const pkg = JSON.parse(readFileSync(newPkgPath, 'utf-8'))
            if (pkg.browser && pkg.main) {
              console.log(`  Removing 'main' field from package.json to force browser version`)
              delete pkg.main
              writeFileSync(newPkgPath, JSON.stringify(pkg, null, 2))
            }
          } catch (err) {
            console.warn(`  Warning: Failed to patch package.json: ${err.message}`)
          }
        }
                // 删除备份
        if (existsSync(extDir + '.backup')) {
          execSync(`rm -rf "${extDir}.backup"`, { stdio: 'pipe' })
        }
        
        console.log(`✓ Installed ${ext.name} with browser support`)
      } else {
        throw new Error('Extension directory not found in VSIX')
      }
      
      // 清理临时文件
      execSync(`rm -rf "${tmpDir}"`, { stdio: 'pipe' })
    } catch (error) {
      console.error(`Failed to download ${ext.name}:`, error.message)
      console.log('Extension may not work properly in Web IDE')
      
      // 如果有备份，恢复它
      if (existsSync(extDir + '.backup')) {
        execSync(`rm -rf "${extDir}" && mv "${extDir}.backup" "${extDir}"`, { stdio: 'pipe' })
      }
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
    createOnigurumaSymlink()
    patchExtensionsForWeb()
    patchQualityAndCommit()
  } else {
    downloadAndExtract()
      .then(() => patchVSCodePackageJson())
      .then(() => downloadLanguagePacks())
      .then(() => downloadMissingWebExtensions())
      .then(() => generateNLSFiles())
      .then(() => createOnigurumaSymlink())
      .then(() => patchExtensionsForWeb())
      .then(() => patchQualityAndCommit())
      .catch(console.error)
  }
}

// export { downloadAndExtract }
