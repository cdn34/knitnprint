import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [tanstackStart(), react()],
  server: {
    port: 3000,
    proxy: {
      '/api': 'http://127.0.0.1:8080',
    },
  },
})
