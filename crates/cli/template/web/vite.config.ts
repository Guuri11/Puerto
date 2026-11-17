import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    // La API responde bajo /api en :8080 (`make run`). Con el proxy, el navegador ve un solo
    // origen: sin CORS en desarrollo.
    proxy: { "/api": "http://localhost:8080" },
  },
});
