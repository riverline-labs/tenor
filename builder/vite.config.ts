import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { resolve } from "path";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [tailwindcss(), wasm(), topLevelAwait(), react()],
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
    },
  },
  build: {
    target: "esnext",
  },
  optimizeDeps: {
    exclude: ["tenor-eval-wasm"],
  },
  define: {
    // Expose TENOR_BUILDER_CONTRACT env var to client code via import.meta.env
    // Set by `tenor builder --contract <path>` via VITE_TENOR_CONTRACT_PATH env var
    "import.meta.env.VITE_TENOR_CONTRACT_PATH": JSON.stringify(
      process.env.VITE_TENOR_CONTRACT_PATH ?? ""
    ),
  },
});
