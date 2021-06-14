const { readFileSync, writeFileSync } = require('fs')
const { execSync } = require('child_process')

const ref = process.env.GITHUB_REF
const tag = ref.split('/')[2]
const version = tag?.startsWith('v') ? tag.substring(1) : tag
const preRelease = version.includes('-')

const changelog = readFileSync('../CHANGELOG.md').toString();
const lines = changelog.split('\n');
const start = lines.findIndex(line => line.startsWith('## ') && line.includes(version)) + 1
if (start == 0) {
    throw new Error('No changelog entry found')
}
const end = lines.slice(start).findIndex(line => line.startsWith('## '))
const range = end == -1 ? lines.slice(start) : lines.slice(start, end)
const notes = range.join('\n') + '\n'
writeFileSync('notes.md', notes)
writeFileSync('content/releases/${tag}.md', `---\ntitle: ${tag}\n---\n${notes}`)
execSync(`gh release create ${tag} -t ${tag} -F notes.md ${preRelease ? '-p' : ''} release/*`)

execSync('git config user.name github-actions')
execSync('git config user.email github-actions@github.com')

if (!preRelease) {
    const latest = {
        version: process.env.INPUT_VERSION,
        notes,
        pub_date: new Date().toISOString(),
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

}

execSync('git add content')
execSync(`git commit -m "chore: release ${tag}"`)
execSync('git push')
