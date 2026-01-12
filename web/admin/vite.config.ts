import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  base: "/admin/",            // <-- чтобы ассеты корректно грузились из /admin/
  build: {
    outDir: "../admin-dist",  // <-- билд сразу в web/admin-dist
    emptyOutDir: true,
  },
});