<template>
  <v-row justify="center" align="center">
    <v-col cols="12" sm="8" md="6">
      <article>
        <v-card>
          <v-card-title>{{ page.title }}</v-card-title>
          <v-card-text>
            <nuxt-content :document="page" />
          </v-card-text>
        </v-card>
      </article>
    </v-col>
  </v-row>
</template>

<script lang="ts">
import Vue from 'vue'
export default Vue.extend({
  async asyncData({ $content }) {
    const page = await $content('home').fetch()

    return {
      page,
    }
  },
})
</script>
