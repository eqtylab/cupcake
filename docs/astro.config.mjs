// @ts-check
import { defineConfig } from "astro/config";
import { fileURLToPath } from "url";
import { resolve } from "path";

import tailwindcss from "@tailwindcss/vite";

import mdx from "@astrojs/mdx";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

// https://astro.build/config
export default defineConfig({
  vite: {
    plugins: [tailwindcss()],
    resolve: {
      alias: {
        "@": resolve(__dirname, "./src"),
        "@/components": resolve(__dirname, "./src/components"),
        "@/lib": resolve(__dirname, "./src/lib"),
        "@/layouts": resolve(__dirname, "./src/layouts"),
        "@/pages": resolve(__dirname, "./src/pages"),
        "@/styles": resolve(__dirname, "./src/styles"),
      },
    },
  },
  integrations: [mdx()],
});
