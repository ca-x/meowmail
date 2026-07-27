import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import { fileViewerRenderers } from "@file-viewer/vite-plugin"

export default defineConfig(({ mode }) => ({
  plugins: [react(), fileViewerRenderers({ copyAssets: mode === "test" ? false : true })],
  server: {
    host: "0.0.0.0",
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:8080",
    },
  },
  build: {
    sourcemap: true,
    target: "es2022",
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
  },
}))
