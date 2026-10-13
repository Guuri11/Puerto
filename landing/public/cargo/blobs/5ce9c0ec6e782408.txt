import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    // The API answers under /api on :8080 (`make run`). Proxied, the browser sees one origin: no
    // CORS in development.
    proxy: { "/api": "http://localhost:8080" },
  },
});
