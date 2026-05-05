import path from "node:path";
import fs from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { visualizer } from "rollup-plugin-visualizer";

// Web-only build entry. The desktop Tauri app continues to use vite.config.ts.
//
// Differences vs vite.config.ts:
// - No code-inspector-plugin (used only in dev for desktop)
// - VITE_TAURI_ENV is hard-coded to false; adapter.isWebMode() relies on the
//   absence of window.__TAURI__ at runtime, but we also expose this for Tree
//   shaking out Tauri-only branches at build time.
// - Output goes to dist-web/ (used by rust-embed).
// - Manual chunks split heavy vendors so the main bundle stays small.
// - Bundle treemap emitted to dist-web/treemap.html for inspection.

export default defineConfig(({ mode }) => {
  const distDir = path.resolve(__dirname, "dist-web");
  const treemapFile = path.join(distDir, "treemap.html");
  fs.mkdirSync(distDir, { recursive: true });

  return {
    root: "src",
  plugins: [
    react(),
    visualizer({
      filename: treemapFile,
      template: "treemap",
      gzipSize: true,
      brotliSize: true,
    }),
  ],
  base: "./",
  define: {
    "import.meta.env.VITE_TAURI_ENV": JSON.stringify("false"),
  },
  build: {
    outDir: distDir,
    emptyOutDir: true,
    target: "es2022",
    sourcemap: mode !== "production",
    rollupOptions: {
      output: {
        manualChunks: {
          codemirror: [
            "@codemirror/state",
            "@codemirror/view",
            "@codemirror/lang-javascript",
            "@codemirror/lang-json",
            "@codemirror/lang-markdown",
            "@codemirror/theme-one-dark",
            "@codemirror/lint",
            "codemirror",
          ],
          recharts: ["recharts"],
          "framer-motion": ["framer-motion"],
          radix: [
            "@radix-ui/react-accordion",
            "@radix-ui/react-checkbox",
            "@radix-ui/react-collapsible",
            "@radix-ui/react-dialog",
            "@radix-ui/react-dropdown-menu",
            "@radix-ui/react-popover",
            "@radix-ui/react-scroll-area",
            "@radix-ui/react-select",
            "@radix-ui/react-switch",
            "@radix-ui/react-tabs",
            "@radix-ui/react-tooltip",
          ],
          tanstack: ["@tanstack/react-query", "@tanstack/react-virtual"],
          dndkit: ["@dnd-kit/core", "@dnd-kit/sortable", "@dnd-kit/utilities"],
          flexsearch: ["flexsearch"],
        },
      },
    },
  },
  server: {
    port: 1421,
    strictPort: true,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_"],
  };
});
