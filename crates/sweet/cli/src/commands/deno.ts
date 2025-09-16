// @ts-nocheck
// deno-lint-ignore-file
// wasm-bindgen deno runner https://vscode.dev/github/rustwasm/wasm-bindgen/blob/main/crates/cli/src/bin/wasm-bindgen-test-runner/deno.rs
import init from "./bindgen.js";

/**
This file is cached and will be replaced on hash change
For more info see [js_runtime_extern.rs](crates/sweet/src/wasm/js_runtime_extern.rs)
**/
globalThis.cwd = () => Deno.cwd();
globalThis.exit = (code: number) => Deno.exit(code);
globalThis.panic_to_error = (f) => f();
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
	} catch (_err) {
		// If --allow-env not granted
		return null;
	}
};

// Expose all env vars as entries [[key, value], ...] to avoid serde on wasm side
globalThis.env_all = () => {
	try {
		return Object.entries(Deno.env.toObject());
	} catch (_err) {
		return [];
	}
};

/// ⚠️ The runner will clear the console in watch mode
const wasm = await init().catch((err: any) => {
	// panicked!
	console.error(err);
	Deno.exit(1);
});

// if run_with_pending doesnt exist this file is being used
// outside of the test runner, no worries i guess, we still
// ran the main(){}
await wasm.run_with_pending?.().catch((err: any) => {
	// panicked!
	console.error(err);
	Deno.exit(1);
});
