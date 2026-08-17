// @ts-nocheck
// deno-lint-ignore-file
//
// The Beet Deno Wasm Runner. A module exporting an async `start` (the beet
// binary) is awaited and the process exits with the returned code; any other
// module (eg a test binary) drives itself through the `exit` global, pending
// timers keeping the process alive until it does.
//
// Includes utilty methods akin to `std::fs`
//
// For more info see [js_runtime.rs](crates/beet_core/src/web_utils/js_runtime.rs)
// for context see how the wasm-bindgen deno runner works
// https://github.com/wasm-bindgen/wasm-bindgen/blob/main/crates/cli/src/wasm_bindgen_test_runner/deno.rs
//
// `.env` is not loaded here: the module does it itself through
// `js_runtime::load_dotenv`, over the `read_file` / `set_env` globals below.
import init, * as bindgen from "./bindgen.js";
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
	return do_try(() => Deno.readFileSync(path), null, true);
};

// List files under `path` recursively, returning each path relative to `path`
// with forward slashes (the beet store list contract). Empty if `path` is absent.
globalThis.read_dir = (path: string) => {
	return do_try(() => {
		const out: string[] = [];
		const walk = (dir: string, prefix: string) => {
			for (const entry of Deno.readDirSync(dir)) {
				const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
				if (entry.isDirectory) {
					walk(`${dir}/${entry.name}`, rel);
				} else {
					out.push(rel);
				}
			}
		};
		walk(path, "");
		return out;
	}, [], true);
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

// Recursively remove a file or directory, returning an error string on failure
// (the `write_file` Option<String> contract). A missing path errors, matching
// `std::fs::remove_*` so `fs_ext::remove` behaves the same on both targets.
globalThis.remove = (path: string) => {
	return do_try_err(() => {
		Deno.removeSync(path, { recursive: true });
	});
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

globalThis.set_env = (key: string, value: string) => {
	return do_try(() => Deno.env.set(String(key), String(value)));
};

globalThis.remove_env = (key: string) => {
	return do_try(() => Deno.env.delete(String(key)));
};

// Expose all env vars as entries [[key, value], ...] to avoid serde on wasm side
// ## Errors
// if --allow-env not granted
globalThis.env_all = () => {
	return do_try(() => Object.entries(Deno.env.toObject()), []);
};

// Test-mode aliases. `beet_core`'s wasm `--lib` tests import these under
// `test_*` names to avoid duplicate wasm-bindgen symbols (see js_runtime.rs).
globalThis.test_cwd = globalThis.cwd;
globalThis.test_exit = globalThis.exit;
globalThis.test_exists = globalThis.exists;
globalThis.test_catch_no_abort_inner = globalThis.catch_no_abort_inner;
globalThis.test_read_file = globalThis.read_file;
globalThis.test_read_dir = globalThis.read_dir;
globalThis.test_create_dir_all = globalThis.create_dir_all;
globalThis.test_write_file = globalThis.write_file;
globalThis.test_remove = globalThis.remove;
globalThis.test_env_args = globalThis.env_args;
globalThis.test_env_var = globalThis.env_var;
globalThis.test_set_env = globalThis.set_env;
globalThis.test_remove_env = globalThis.remove_env;
globalThis.test_env_all = globalThis.env_all;

const _wasm = await init().catch((err: any) => {
	// panicked!
	console.error(err);
	Deno.exit(1);
});

// A test module exports `test_start` (the `test_main!` macro), the same
// awaited-async-start contract as an app's `start` under a collision-proof
// name: await the suite, exit with its verdict.
if (typeof bindgen.test_start === "function") {
	Deno.exit((await bindgen.test_start()) ?? 0);
}

// A module exporting an async `start` (the beet binary) resolves to its exit
// code: await it and exit with the code, the one-shot shape where the code is a
// return value rather than a side channel.
if (typeof bindgen.start === "function") {
	Deno.exit((await bindgen.start()) ?? 0);
}

// Otherwise the module is a daemon: it booted from `init()` (wasm-bindgen calls
// its `main`), drives its own tasks, and terminates by calling the `exit` global.
// Beet's wasm executor is not a JS macrotask, so the event loop can drain while
// the module still has work; this keepalive holds the process open until it
// exits itself. `start` is therefore a pure opt-in, never a requirement.
await keep_alive();

//-- Helpers --

// A try-catch wrapper that will log the error and return on_err
// if an exception is raised. Useful for wasm wrappers where
// we still want to return something, like None or empty array
function do_try<Ok, Err = null>(
	func: () => Ok,
	on_err: Err = null,
	silent: boolean = false,
): Ok | Err {
	try {
		return func();
	} catch (err) {
		// If --allow-env not granted
		if (!silent) {
			console.error(err);
		}
		return on_err;
	}
}
// Run a fallible side-effect, returning null on success or the error string on
// throw: the `Option<String>` contract the Rust `write_file`/`remove` bindings read.
function do_try_err(func: () => void): string | null {
	try {
		func();
		return null;
	} catch (err) {
		return String(err);
	}
}
// Hold the deno event loop open indefinitely, for a module that terminates by
// calling `globalThis.exit` rather than by resolving a `start` export.
async function keep_alive() {
	while (true) {
		await new Promise((resolve) => setTimeout(resolve, 1_000));
	}
}
