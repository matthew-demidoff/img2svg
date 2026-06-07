import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The wasm core lives outside this package (crates/img2svg-core/pkg) and is
// loaded at runtime via a dynamic import, so it is never resolved at build time.
export default defineConfig({
  plugins: [react()],
  worker: {
    format: "es",
  },
});
