import Vue from 'vue'
import Vuetify from 'vuetify'
import { config } from '@vue/test-utils'
import NuxtContent from './mocks/NuxtContent.vue'

Vue.use(Vuetify)
config.stubs['nuxt-content'] = NuxtContent
