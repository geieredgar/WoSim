const defaultVersion = process.env.INPUT_VERSION
const msiVersion = process.env.INPUT_MSI_VERSION

enum Platform {
    Linux = 'linux',
    Windows = 'win32',
    MacOs = 'darwin'
}

const platform = process.platform as Platform

const productName = (() => {
    switch (platform) {
        case Platform.Linux: return 'wosim-hub'
        default: return 'WoSim Hub'
    }
})();

const version = (() => {
    switch (platform) {
        case Platform.Windows: return msiVersion
        default: return defaultVersion
    }
})();

const pubkey = process.env.TAURI_PUBLIC_KEY

const config = {
    package: {
        productName,
        version
    },
    tauri: {
        updater: {
            pubkey
        }
    }
}

const src = (() => {
    switch (platform) {
        case Platform.Linux: return `src-tauri/target/release/bundle/appimage/wosim-hub_${version}_amd64.AppImage`
        case Platform.MacOs: return `src-tauri/target/release/bundle/macos/WoSim Hub.app`
        case Platform.Windows: return `src-tauri\\target\\release\\bundle\\msi\\WoSim Hub_${version}_x64.msi`
    }
})()

const dst = (() => {
    switch (platform) {
        case Platform.Linux: return `release/wosim-hub-linux-amd64.AppImage`
        case Platform.MacOs: return `release/wosim-hub-macos-x64.app`
        case Platform.Windows: return `release\\wosim-hub-windows-x64.msi`
    }
})()

const archive = (() => {
    switch (platform) {
        case Platform.Windows: return 'zip'
        default: return 'tar.gz'
    }
})()

const { execSync } = require('child_process')

execSync(`yarn tauri build -c "${JSON.stringify(config).replace(/"/g, '\\"')}"`, { stdio: 'inherit' })

if (pubkey !== undefined) {
    const { copyFileSync, mkdirSync } = require('fs')

    mkdirSync('release')
    if (platform !== Platform.MacOs) {
        copyFileSync(src, dst)
    } else {
        copyFileSync(`src-tauri/target/release/bundle/dmg/WoSim Hub_${version}_x64.dmg`, 'release/wosim-hub-macos-x64.dmg')
    }
    copyFileSync(`${src}.${archive}`, `${dst}.${archive}`)
    copyFileSync(`${src}.${archive}.sig`, `${dst}.${archive}.sig`)
}
