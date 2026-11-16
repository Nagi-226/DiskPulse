import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
// @ts-expect-error process is a nodejs global
const appVersion = process.env.npm_package_version ?? "0.8.0";

export default defineConfig(async ({ mode }) => {
  const isWebDashboardBuild = mode === "web-dashboard";

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: isWebDashboardBuild
        ? {
            "@tauri-apps/api/core": fileURLToPath(new URL("./src/api/index.ts", import.meta.url)),
            "@tauri-apps/api/event": fileURLToPath(
              new URL("./src/api/tauri-event-web.ts", import.meta.url),
            ),
            "@tauri-apps/plugin-notification": fileURLToPath(
              new URL("./src/api/notification-web.ts", import.meta.url),
            ),
          }
        : undefined,
    },
    define: {
      __APP_VERSION__: JSON.stringify(appVersion),
    },
    build: {
      outDir: isWebDashboardBuild ? "dist-web-dashboard" : "dist",
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes("node_modules/react") || id.includes("node_modules/react-dom")) {
              return "vendor-react";
            }
            if (id.includes("node_modules/echarts") || id.includes("node_modules/zrender")) {
              return "vendor-echarts";
            }
            if (id.includes("node_modules/lucide-react")) {
              return "vendor-ui";
            }
          },
        },
      },
    },

    clearScreen: false,
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
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
