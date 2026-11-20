import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  // El .env de la raíz del proyecto, el mismo que lee la API.
  const env = loadEnv(mode, "..", "");
  return {
    plugins: [react(), tailwindcss()],
    server: {
      port: 5173,
      // La API responde bajo /api en SERVICE_PORT (`make run`). Con el proxy, el navegador ve un
      // solo origen: sin CORS en desarrollo.
      proxy: { "/api": `http://localhost:${env.SERVICE_PORT || "8080"}` },
    },
  };
});
