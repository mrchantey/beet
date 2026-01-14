// @ts-nocheck
// deno-lint-ignore-file
//
// The Beet Deno Wasm Runner, runs the provided
// wasm binary until it calls `js_runtime::exit`
//
// Includes utilty methods akin to `std::fs`
//
// This file is cached and will be replaced on hash change
// For more info see [js_runtime.rs](crates/beet_core/src/wasm_utils/js_runtime.rs)
// for context see how the wasm-bindgen deno runner works
// https://github.com/wasm-bindgen/wasm-bindgen/blob/main/crates/cli/src/wasm_bindgen_test_runner/deno.rs
import init from "./bindgen.js";
import { dirname } from "https://deno.land/std/path/mod.ts";
import { ensureDirSync, existsSync } from "https://deno.land/std/fs/mod.ts";

globalThis.cwd = () => {
	return do_try(() => Deno.cwd());
};
globalThis.exit = (code: number) => {
	return do_try(() => Deno.exit(code));
};
globalThis.catch_no_abort_inner = (func: () => undefined) => {
	return func();
};
globalThis.read_file = (path: string) => {
	return do_try(() => Deno.readFileSync(path));
};

globalThis.exists = (path: string) => {
	return do_try(() => existsSync(path), false);
};
globalThis.create_dir_all = (path: string) => {
	return do_try(() => ensureDirSync(path));
};
globalThis.write_file = (path: string, content: Uint8Array) => {
	return do_try(() => Deno.writeFileSync(path, content));
};

globalThis.env_args = () => {
	return do_try(() => Deno.args, []);
};

// Expose single env var (maps undefined -> null for wasm-bindgen Option)
// ## Errors
// if --allow-env not granted
globalThis.env_var = (key: string) => {
	return do_try(() => Deno.env.get(String(key)) ?? null);
};

// Expose all env vars as entries [[key, value], ...] to avoid serde on wasm side
// ## Errors
// if --allow-env not granted
globalThis.env_all = () => {
	return do_try(() => Object.entries(Deno.env.toObject()), []);
};

// Test mode aliases (when running cargo test -p beet_core)
globalThis.test_cwd = globalThis.cwd;
globalThis.test_exit = globalThis.exit;
globalThis.test_exists = globalThis.exists;
globalThis.test_catch_no_abort_inner = globalThis.catch_no_abort_inner;
globalThis.test_read_file = globalThis.read_file;
globalThis.test_create_dir_all = globalThis.create_dir_all;
globalThis.test_write_file = globalThis.write_file;
globalThis.test_env_args = globalThis.env_args;
globalThis.test_env_var = globalThis.env_var;
globalThis.test_env_all = globalThis.env_all;

const _wasm = await init().catch((err: any) => {
	// panicked!
	console.error(err);
	Deno.exit(1);
});

/// Keep the process alive, JsRuntimePlugin will decide when to exit
await loop_forever();

//-- Helpers --

// A try-catch wrapper that will log the error and return on_err
// if an exception is raised. Useful for wasm wrappers where
// we still want to return something, like None or empty array
function do_try<Ok, Err = null>(func: () => Ok, on_err: Err = null): Ok | Err {
	try {
		return func();
	} catch (err) {
		// If --allow-env not granted
		console.error(err);
		return on_err;
	}
}
async function loop_forever() {
	while (true) {
		await new Promise((resolve) => setTimeout(resolve, 1_000));
	}
}
