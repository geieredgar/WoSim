import fs from 'fs'
import path from 'path'
import { $content } from '@nuxt/content'

export default {
  target: 'static',

  modules: ['@nuxt/content'],

  content: {},

  hooks: {
    generate: {
      async distCopied(generator) {
        const entries = await $content('/', { deep: true })
          .sortBy('date', 'desc')
          .fetch()
        entries.forEach((entry) => {
          fs.mkdirSync(path.join(generator.distPath, 'content', entry.dir), {
            recursive: true,
          })
          fs.writeFileSync(
            path.join(generator.distPath, 'content', `${entry.path}.json`),
            JSON.stringify(entry)
          )
          fs.appendFileSync(
            path.join(generator.distPath, 'content', entry.dir, 'index.txt'),
            `${entry.path.substring(entry.dir.length + 1)}.json\n`
          )
        })
      },
      done(generator) {
        fs.rmSync(path.join(generator.distPath, '_nuxt'), { recursive: true })
      },
    },
  },
}
