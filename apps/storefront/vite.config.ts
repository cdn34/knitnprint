import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import react from '@vitejs/plugin-react'
import { nitro } from 'nitro/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    tanstackStart(),
    nitro({
      devProxy: {
        '/api/**': {
          target: process.env.API_BASE_URL ?? 'http://127.0.0.1:8080',
          changeOrigin: true,
        },
      },
    }),
    react(),
  ],
  server: {
    host: '127.0.0.1',
    port: 3000,
    strictPort: true,
  },
})
