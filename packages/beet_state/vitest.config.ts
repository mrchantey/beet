import wasm from "vite-plugin-wasm";
import { defineConfig } from "vitest/config";

export default defineConfig({
	plugins: [wasm()],
	test: {
		environment: "happy-dom",
		globals: true,
	},
});
