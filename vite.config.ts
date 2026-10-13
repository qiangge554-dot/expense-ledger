import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// Tauri 期望固定的 5173 端口；clearScreen=false 以保留 Rust 编译输出
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: false,
  },
  // Tauri 在生产构建时使用相对资源路径
  base: "./",
});
