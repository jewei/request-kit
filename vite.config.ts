import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

// Tauri expects a fixed dev-server port and no screen clearing so its own
// output stays visible.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'es2022',
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
