//! Runtime bindings for JavaScript environments (Deno test runner, browser,
//! Cloudflare Worker).
//!
//! This module provides FFI bindings to host JavaScript APIs for file I/O,
//! environment variables, args and panic handling. The underlying functions
//! (`env_var`, `read_file`, ...) are provided as globals by the beet Deno test
//! runner; a browser or a Cloudflare Worker has no such globals.
//!
//! Every binding is therefore wrapped in a presence-check ([`has_global`]): when
//! the host does not define the global the wrapper returns a safe default
//! (`None`, empty, a no-op) instead of throwing a `ReferenceError`, which on wasm
//! traps the module and hangs the caller. So a served wasm Worker degrades to
//! "no process env / no fs" rather than crashing, while the Deno runner keeps its
//! full surface.

use crate::prelude::*;
use wasm_bindgen::prelude::*;

// The raw host globals. Names are load-bearing: `beet_core` dev-depends on itself
// (the `testing` feature), so a `--lib` wasm test links it twice — once under
// `cfg(test)` and once as a plain rlib. wasm-bindgen derives its
// `__wbindgen_describe___wbg_<name>` symbols from the js name, so identical names
// across the two builds collide. The `cfg(test)` build uses distinct `test_*`
// names (aliased in `deno.ts`) to avoid the collision.
#[cfg(not(test))]
mod raw {
	use wasm_bindgen::prelude::*;
	#[wasm_bindgen]
	unsafe extern "C" {
		pub fn cwd() -> String;
		pub fn exit(code: i32);
		#[wasm_bindgen(catch)]
		pub fn catch_no_abort_inner(
			f: &mut dyn FnMut() -> Result<(), String>,
		) -> Result<(), JsValue>;
		pub fn read_file(path: &str) -> Option<Vec<u8>>;
		pub fn create_dir_all(path: &str);
		pub fn exists(path: &str) -> bool;
		pub fn write_file(path: &str, content: &[u8]) -> Option<String>;
		pub fn env_args() -> js_sys::Array;
		pub fn env_var(key: &str) -> Option<String>;
		pub fn set_env(key: &str, value: &str);
		pub fn env_all() -> js_sys::Array;
	}
}

#[cfg(test)]
mod raw {
	use wasm_bindgen::prelude::*;
	#[wasm_bindgen]
	unsafe extern "C" {
		#[wasm_bindgen(js_name = "test_cwd")]
		pub fn cwd() -> String;
		#[wasm_bindgen(js_name = "test_exit")]
		pub fn exit(code: i32);
		#[wasm_bindgen(catch, js_name = "test_catch_no_abort_inner")]
		pub fn catch_no_abort_inner(
			f: &mut dyn FnMut() -> Result<(), String>,
		) -> Result<(), JsValue>;
		#[wasm_bindgen(js_name = "test_read_file")]
		pub fn read_file(path: &str) -> Option<Vec<u8>>;
		#[wasm_bindgen(js_name = "test_exists")]
		pub fn exists(path: &str) -> bool;
		#[wasm_bindgen(js_name = "test_create_dir_all")]
		pub fn create_dir_all(path: &str);
		#[wasm_bindgen(js_name = "test_write_file")]
		pub fn write_file(path: &str, content: &[u8]) -> Option<String>;
		#[wasm_bindgen(js_name = "test_env_args")]
		pub fn env_args() -> js_sys::Array;
		#[wasm_bindgen(js_name = "test_env_var")]
		pub fn env_var(key: &str) -> Option<String>;
		#[wasm_bindgen(js_name = "test_set_env")]
		pub fn set_env(key: &str, value: &str);
		#[wasm_bindgen(js_name = "test_env_all")]
		pub fn env_all() -> js_sys::Array;
	}
}

/// The js-name prefix the host globals are bound under: `test_*` in a `--lib`
/// wasm test (see [`raw`]), bare otherwise. Used by [`has_global`] to probe the
/// right name on `globalThis`.
#[cfg(test)]
const PREFIX: &str = "test_";
#[cfg(not(test))]
const PREFIX: &str = "";

/// Whether the host defines a callable global of the given (un-prefixed) name,
/// eg `env_var`. False in a browser / Worker, where the Deno runner globals are
/// absent, so the wrappers below can fall back instead of trapping.
fn has_global(name: &str) -> bool {
	let key = JsValue::from_str(&format!("{PREFIX}{name}"));
	js_sys::Reflect::get(&js_sys::global(), &key)
		.map(|value| value.is_function())
		.unwrap_or(false)
}

/// Current working directory, ie `Deno.cwd()`. `/` when the host has no cwd
/// (browser / Worker), matching a rooted virtual filesystem.
pub fn cwd() -> String {
	if has_global("cwd") {
		raw::cwd()
	} else {
		"/".to_string()
	}
}

/// Exit the runtime with a code, ie `Deno.exit()`. A no-op where unavailable (a
/// Worker has no process to exit).
pub fn exit(code: i32) {
	if has_global("exit") {
		raw::exit(code);
	}
}

/// Read a file, ie `Deno.readFileSync()`. `None` where the fs global is absent.
pub fn read_file(path: &str) -> Option<Vec<u8>> {
	if has_global("read_file") {
		raw::read_file(path)
	} else {
		None
	}
}

/// Ensure a directory exists, ie `Deno.mkdirSync()`. A no-op where unavailable.
pub fn create_dir_all(path: &str) {
	if has_global("create_dir_all") {
		raw::create_dir_all(path);
	}
}

/// Whether a path exists, ie `Deno.existsSync()`. `false` where unavailable.
pub fn exists(path: &str) -> bool {
	if has_global("exists") {
		raw::exists(path)
	} else {
		false
	}
}

/// Write a file, ie `Deno.writeFileSync()`, returning an error string on
/// failure. A no-op where the fs global is absent.
pub fn write_file(path: &str, content: &[u8]) -> Option<String> {
	if has_global("write_file") {
		raw::write_file(path, content)
	} else {
		None
	}
}

/// Command-line args excluding the program name, ie `Deno.args`. Empty where
/// unavailable (a Worker has no argv).
pub fn env_args() -> js_sys::Array {
	if has_global("env_args") {
		raw::env_args()
	} else {
		js_sys::Array::new()
	}
}

/// A single environment variable, ie `Deno.env.get(key)`. `None` where the env
/// global is absent (a Worker's config is its `worker::Env` bindings, not a
/// process environment).
pub fn env_var(key: &str) -> Option<String> {
	if has_global("env_var") {
		raw::env_var(key)
	} else {
		None
	}
}

/// Set an environment variable, ie `Deno.env.set(key, value)`. A no-op where
/// unavailable.
pub fn set_env(key: &str, value: &str) {
	if has_global("set_env") {
		raw::set_env(key, value);
	}
}

/// All environment variables as a 2D entries array, ie
/// `Object.entries(Deno.env.toObject())`. Empty where unavailable.
pub fn env_all() -> js_sys::Array {
	if has_global("env_all") {
		raw::env_all()
	} else {
		js_sys::Array::new()
	}
}

/// Runs a function and catches panics without aborting the wasm module.
///
/// This provides a wasm-compatible alternative to [`std::panic::catch_unwind`],
/// executing the function in JavaScript to catch panics gracefully. Where the
/// host has no `catch_no_abort_inner` global the function is run directly (a
/// panic then traps, as there is no JS frame to catch it).
///
/// Returns `Ok(Ok(()))` on success, `Ok(Err(msg))` if the function returned
/// an error string, or `Err(())` if a panic occurred.
pub fn catch_no_abort(
	func: impl FnOnce() -> Result<(), String>,
) -> Result<Result<(), String>, ()> {
	if !has_global("catch_no_abort_inner") {
		// no JS catch frame available: run directly and map the error string.
		return Ok(func());
	}
	let mut opt = Some(func);
	let outcome = raw::catch_no_abort_inner(&mut || {
		opt.take().expect("function already called")()
	});
	match outcome {
		Ok(()) => Ok(Ok(())),
		Err(err) if err.is_string() => {
			Ok(Err(err.as_string().expect("checked")))
		}
		Err(_) => Err(()),
	}
}
