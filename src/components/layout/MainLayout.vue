<script setup lang="ts">
// M2a: sidebar | main grid. TabsBar arrives with multi-tab in M3a.
import RequestView from '../request/RequestView.vue';
import Sidebar from './Sidebar.vue';
</script>

<template>
  <div class="main-layout">
    <Sidebar />
    <main class="main-pane">
      <RequestView />
    </main>
  </div>
</template>

<style scoped>
.main-layout {
  height: 100vh;
  display: grid;
  grid-template-columns: 260px 1fr;
  overflow: hidden;
}
.main-pane {
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
