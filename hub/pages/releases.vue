<template>
  <v-row justify="center" align="center">
    <v-col cols="12" sm="8" md="6">
      <article>
        <v-card v-for="page in pages" :key="page.title">
          <v-card-title
            >{{ page.title }}
            <v-spacer></v-spacer>
            <v-chip
              v-if="page.preRelease"
              disabled
              small
              pill
              outlined
              color="warning"
              >Pre-release</v-chip
            ></v-card-title
          >
          <v-card-subtitle
            >released
            {{ new Date(page.publishedAt) | moment('from') }}
          </v-card-subtitle>
          <v-card-text>
            <nuxt-content :document="page" />
          </v-card-text>
          <v-card-actions>
            <v-btn
              @click="
                open(
                  `https://github.com/wosim-net/hub/releases/tag/${page.slug}`
                )
              "
              >View at GitHub</v-btn
            >
          </v-card-actions>
        </v-card>
      </article>
    </v-col>
  </v-row>
</template>

<script lang="ts">
import Vue from 'vue'
import { open } from '@tauri-apps/api/shell'
import { releaseNotes } from '~/api/releases'

export default Vue.extend({
  async asyncData() {
    const pages = await releaseNotes()
    return {
      pages,
    }
  },
  methods: {
    open(url: string) {
      open(url)
    },
  },
})
</script>
