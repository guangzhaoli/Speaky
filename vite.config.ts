import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  // 多页面入口配置
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        indicator: resolve(__dirname, "indicator.html"),
      },
      output: {
        // 优化 chunk 分割
        manualChunks: {
          // 将 React 相关放在一起
          react: ["react", "react-dom"],
          // Tauri 插件单独 chunk
          tauri: [
            "@tauri-apps/api",
            "@tauri-apps/plugin-clipboard-manager",
            "@tauri-apps/plugin-global-shortcut",
          ],
        },
      },
    },
    // 启用最小化
    minify: "esbuild",
    // 减少 chunk 大小警告阈值
    chunkSizeWarningLimit: 500,
    // 生成更小的文件
    target: "esnext",
    // 不生成 sourcemap (生产环境)
    sourcemap: false,
  },

  // 优化依赖预构建
  optimizeDeps: {
    include: ["react", "react-dom"],
  },

  // esbuild 优化
  esbuild: {
    // 移除 console 和 debugger (生产环境)
    drop: process.env.NODE_ENV === "production" ? ["console", "debugger"] : [],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
