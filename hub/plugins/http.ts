import { Plugin } from '@nuxt/types'
import { Client, getClient } from '@tauri-apps/api/http'

declare module '@nuxt/types' {
  interface Context {
    $http: Client
  }

  interface NuxtAppOptions {
    $http: Client
  }
}

declare module 'vue/types/vue' {
  interface Vue {
    $http: Client
  }
}

const plugin: Plugin = async (_context, inject) => {
  inject('http', await getClient())
}

export default plugin
