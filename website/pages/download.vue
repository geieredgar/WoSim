<template>
  <v-row justify="center" align="center">
    <v-col cols="12" sm="6" md="4" lg="3">
      <v-card>
        <v-card-title
          ><v-icon large left>mdi-linux</v-icon
          ><span>Linux (amd64, AppImage)</span></v-card-title
        >
        <v-card-actions>
          <v-btn
            href="https://github.com/wosim-net/hub/releases/latest/download/wosim-hub-linux-amd64.AppImage"
            >Download
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-col>
    <v-col cols="12" sm="6" md="4" lg="3">
      <v-card>
        <v-card-title
          ><v-icon large left>mdi-microsoft-windows</v-icon
          ><span>Windows (x64)</span></v-card-title
        >
        <v-card-actions>
          <v-btn
            href="https://github.com/wosim-net/hub/releases/latest/download/wosim-hub-windows-x64.msi"
            >Download
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-col>
    <v-col cols="12" sm="6" md="4" lg="3">
      <v-card>
        <v-card-title>
          <v-icon large left>mdi-apple</v-icon
          ><span>macOS (x64)</span></v-card-title
        >
        <v-card-actions
          ><v-btn
            href="https://github.com/wosim-net/hub/releases/latest/download/wosim-hub-macos-x64.dmg"
            >Download
          </v-btn></v-card-actions
        >
      </v-card>
    </v-col>
  </v-row>
</template>
<script lang="ts">
import Vue from 'vue'
export default Vue.extend({
  head() {
    return {
      title: 'Download Hub',
    }
  },
})
</script>
