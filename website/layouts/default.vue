<template>
  <v-app dark>
    <v-navigation-drawer v-model="drawer" clipped fixed app>
      <v-list>
        <v-list-item
          v-for="(item, i) in items"
          :key="i"
          :to="item.to"
          router
          exact
        >
          <v-list-item-action>
            <v-icon>{{ item.icon }}</v-icon>
          </v-list-item-action>
          <v-list-item-content>
            <v-list-item-title v-text="item.title" />
          </v-list-item-content>
        </v-list-item>
      </v-list>
    </v-navigation-drawer>
    <v-app-bar clipped-left fixed app>
      <v-app-bar-nav-icon @click.stop="drawer = !drawer" />
      <v-toolbar-title class="wosim-title" to="/"
        ><router-link tag="span" to="/">WoSim</router-link></v-toolbar-title
      >
      <v-spacer></v-spacer>
      <v-btn icon href="https://github.com/wosim-net">
        <v-icon x-large>mdi-github</v-icon>
      </v-btn>
    </v-app-bar>
    <v-main>
      <v-container>
        <nuxt />
      </v-container>
    </v-main>
    <v-footer absolute app>
      <v-col class="text-center" cols="12"> &copy; 2021 </v-col>
    </v-footer>
  </v-app>
</template>

<script lang="ts">
export default {
  data() {
    return {
      drawer: false,
      items: [
        {
          icon: 'mdi-home',
          title: 'Home',
          to: '/',
        },
        {
          icon: 'mdi-download',
          title: 'Download Hub',
          to: '/download',
        },
      ],
    }
  },
}
</script>
<style lang="scss" scoped>
.wosim-title {
  font-size: 2rem;
  font-family: 'Leckerli One', cursive;
}
</style>
