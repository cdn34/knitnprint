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
          target: 'http://127.0.0.1:8080',
          changeOrigin: true,
        },
      },
    }),
    react(),
  ],
  server: {
    port: 3000,
  },
})
