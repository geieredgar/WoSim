const tag = process.env.GITHUB_REF?.split('/')[2]
const fullVersion = tag?.startsWith('v') ? tag.substring(1) : tag
const preRelease = fullVersion?.includes('-') || false

enum Platform {
  Linux = 'linux',
  Windows = 'win32',
  MacOs = 'darwin',
}

const platform = process.platform as Platform

const productName = (() => {
  switch (platform) {
    case Platform.Linux:
      return 'wosim-hub'
    default:
      return 'WoSim Hub'
  }
})()

const version = (() => {
  switch (platform) {
    case Platform.Windows:
      return fullVersion?.split('-')[0]
    default:
      return fullVersion
  }
})()

const pubkey = process.env.TAURI_PUBLIC_KEY

const config: any = {
  package: {
    productName,
    version,
  },
}

if (!preRelease) {
  config.tauri = {
    updater: {
      active: true,
      endpoints: ['https://wosim.net/hub/latest.json'],
      pubkey,
    },
  }
}

const src = (() => {
  switch (platform) {
    case Platform.Linux:
      return `src-tauri/target/release/bundle/appimage/wosim-hub_${version}_amd64.AppImage`
    case Platform.MacOs:
      return `src-tauri/target/release/bundle/macos/WoSim Hub.app`
    case Platform.Windows:
      return `src-tauri\\target\\release\\bundle\\msi\\WoSim Hub_${version}_x64.msi`
  }
})()

const dst = (() => {
  switch (platform) {
    case Platform.Linux:
      return `release/wosim-hub-linux-amd64.AppImage`
    case Platform.MacOs:
      return `release/wosim-hub-macos-x64.app`
    case Platform.Windows:
      return `release\\wosim-hub-windows-x64.msi`
  }
})()

const archive = (() => {
  switch (platform) {
    case Platform.Windows:
      return 'zip'
    default:
      return 'tar.gz'
  }
})()

const { execSync } = require('child_process')

execSync(
  `yarn tauri build -c "${JSON.stringify(config).replace(/"/g, '\\"')}"`,
  { stdio: 'inherit' }
)

if (pubkey !== undefined) {
  const { copyFileSync, mkdirSync } = require('fs')

  mkdirSync('release')
  if (platform !== Platform.MacOs) {
    copyFileSync(src, dst)
  } else {
    copyFileSync(
      `src-tauri/target/release/bundle/dmg/WoSim Hub_${version}_x64.dmg`,
      'release/wosim-hub-macos-x64.dmg'
    )
  }
  if (!preRelease) {
    copyFileSync(`${src}.${archive}`, `${dst}.${archive}`)
    copyFileSync(`${src}.${archive}.sig`, `${dst}.${archive}.sig`)
  }
}
