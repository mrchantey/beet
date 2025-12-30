// @ts-nocheck
// deno-lint-ignore-file
//
// The Beet Deno Wasm Runner
//
// This file is cached and will be replaced on hash change
// For more info see [js_runtime_extern.rs](crates/sweet/src/wasm/js_runtime_extern.rs)
// for context see how the wasm-bindgen deno runner works
// https://github.com/wasm-bindgen/wasm-bindgen/blob/main/crates/cli/src/wasm_bindgen_test_runner/deno.rs
//
import init from "./bindgen.js";

globalThis.cwd = () => Deno.cwd();
globalThis.exit = (code: number) => Deno.exit(code);
globalThis.catch_no_abort_inner = (func: () => undefined) => func();
globalThis.read_file = (path: string) => {
	try {
		return Deno.readTextFileSync(path);
	} catch (err) {
		return null;
	}
};
globalThis.sweet_root = () => Deno.env.get("SWEET_ROOT");

// Expose single env var (maps undefined -> null for wasm-bindgen Option)
globalThis.env_var = (key: string) => {
	try {
		return Deno.env.get(String(key)) ?? null;
	} catch (err) {
		// If --allow-env not granted
		console.error(err);
		return null;
	}
};

// Expose all env vars as entries [[key, value], ...] to avoid serde on wasm side
globalThis.env_all = () => {
	try {
		return Object.entries(Deno.env.toObject());
	} catch (err) {
		// If --allow-env not granted
		console.error(err);
		return [];
	}
};

/// ⚠️ The runner will clear the console in watch mode
const wasm = await init().catch((err: any) => {
	// panicked!
	console.error(err);
	Deno.exit(1);
});

/// Keep the process alive, JsRuntimePlugin will decide when to exit
await (async () => {
	while (true) {
		await new Promise((resolve) => setTimeout(resolve, 1_000));
	}
})();
