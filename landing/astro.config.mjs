// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import sitemap from "@astrojs/sitemap";

export default defineConfig({
  site: "https://puerto-rs.vercel.app",
  vite: {
    plugins: [tailwindcss()],
  },
  integrations: [sitemap()],
});
