<script setup lang="ts">
// M1: a single full-window RequestView. Sidebar + TabsBar arrive in M2/M3a,
// at which point this becomes the sidebar | main grid described in PLAN.md.
import RequestView from '../request/RequestView.vue';
</script>

<template>
  <div class="main-layout">
    <RequestView />
  </div>
</template>

<style scoped>
.main-layout {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
