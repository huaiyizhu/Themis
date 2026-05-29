import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        diagnose: resolve(__dirname, "diagnose.html"),
        mini: resolve(__dirname, "mini.html"),
      },
    },
  },
});
