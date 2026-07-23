import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import { showMainWindow, storageRoot } from './ipc/commands';
import './styles/base.css';
import './styles/themes.css';

const app = createApp(App);
app.use(createPinia());
app.mount('#app');

// The window starts hidden (tauri.conf.json visible:false) so window-state can
// restore geometry before anything is painted; reveal once Vue has mounted.
showMainWindow().catch((err) => {
  console.error('failed to show main window', err);
});

// First-run bootstrap: ensures ~/.request-kit exists with restrictive permissions.
storageRoot().catch((err) => {
  console.error('failed to ensure storage root', err);
});
