import { defineConfig, loadEnv } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env: Record<string, string> = loadEnv(mode, ".", "");
  return {
    plugins: [tailwindcss(), react()],
    server: {
      host: true,
      proxy: {
        "/api": {
          target: env.VITE_BACKEND_URL ?? "http://localhost:3000",
          changeOrigin: true,
        },
      },
    },
  };
});
