const { readFileSync, writeFileSync } = require('fs')
const { execSync } = require('child_process')

const release = JSON.parse(execSync(`gh release view v${process.env.INPUT_VERSION} --json body --json publishedAt`))

const latest = {
    version: process.env.INPUT_VERSION,
    notes: release.body,
    pub_date: new Date(release.publishedAt).toISOString(),
    platforms: {
        darwin: {
            signature:
                readFileSync('release/wosim-hub-macos-x64.app.tar.gz.sig').toString(),
            url: `https://github.com/wosim-net/hub/releases/download/v${process.env.INPUT_VERSION}/wosim-hub-macos-x64.app.tar.gz`
        },
        linux: {
            signature:
                readFileSync('release/wosim-hub-linux-amd64.AppImage.tar.gz.sig').toString(),
            url: `https://github.com/wosim-net/hub/releases/download/v${process.env.INPUT_VERSION}/wosim-hub-linux-amd64.AppImage.tar.gz`
        },
        win64: {
            signature:
                readFileSync('release/wosim-hub-windows-x64.msi.zip.sig').toString(),
            url: `https://github.com/wosim-net/hub/releases/download/v${process.env.INPUT_VERSION}/wosim-hub-windows-x64.msi.zip`
        }
    }
}

writeFileSync('static/latest.json', JSON.stringify(latest))
execSync('git add static/latest.json')
execSync('git commit -m "chore: update latest.json"')
execSync('git push')
