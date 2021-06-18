import colors from 'vuetify/es5/util/colors'

export default {
  target: 'static',

  head: {
    titleTemplate: '%s - WoSim',
    title: 'WoSim',
    htmlAttrs: {
      lang: 'en',
    },
    meta: [
      { charset: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      { hid: 'description', name: 'description', content: '' },
    ],
    link: [
      { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
      { rel: 'preconnect', href: 'https://fonts.gstatic.com' },
      {
        rel: 'stylesheet',
        href: 'https://fonts.googleapis.com/css2?family=Leckerli+One&display=swap',
      },
    ],
  },

  css: [],

  plugins: [],

  components: true,

  buildModules: [
    '@nuxt/typescript-build',
    '@nuxtjs/stylelint-module',
    '@nuxtjs/vuetify',
  ],

  modules: ['@nuxt/content'],

  content: {},

  vuetify: {
    theme: {
      dark: true,
      themes: {
        dark: {
          primary: colors.blue.darken2,
          accent: colors.grey.darken3,
          secondary: colors.amber.darken3,
          info: colors.teal.lighten1,
          warning: colors.amber.base,
          error: colors.deepOrange.accent4,
          success: colors.green.accent3,
        },
      },
    },
  },

  build: {},

  hooks: {
    generate: {
      async distCopied(generator) {
        const entries = await $content('/hub', { deep: true })
          .sortBy('date', 'desc')
          .fetch()
        entries.forEach((entry) => {
          fs.mkdirSync(path.join(generator.distPath, 'hub', entry.dir), {
            recursive: true,
          })
          fs.writeFileSync(
            path.join(generator.distPath, 'hub', `${entry.path}.json`),
            JSON.stringify(entry)
          )
          fs.appendFileSync(
            path.join(generator.distPath, 'hub', entry.dir, 'index.txt'),
            `${entry.path.substring(entry.dir.length + 1)}.json\n`
          )
        })
      },
    },
  },
}
