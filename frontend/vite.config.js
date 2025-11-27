import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    allowedHosts: ['spellcast2.twkr.io'],
    port: 3000,
    proxy: {
      '/ws': {
        target: 'ws://localhost:3001',
        ws: true,
      },
      '/api': {
        target: 'http://localhost:3001',
      },
      '/health': {
        target: 'http://localhost:3001',
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
  },
});
