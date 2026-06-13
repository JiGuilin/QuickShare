import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5174,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://localhost:53318',
        changeOrigin: true,
      },
    },
  },
  build: {
    target: ["es2021", "chrome100", "safari13"],
    minify: false,
    sourcemap: true,
    outDir: "dist",
  },
});
