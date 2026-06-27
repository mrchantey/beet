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
		pub fn read_dir(path: &str) -> js_sys::Array;
		pub fn create_dir_all(path: &str);
		pub fn exists(path: &str) -> bool;
		pub fn write_file(path: &str, content: &[u8]) -> Option<String>;
		pub fn remove(path: &str) -> Option<String>;
		pub fn env_args() -> js_sys::Array;
		pub fn env_var(key: &str) -> Option<String>;
		pub fn set_env(key: &str, value: &str);
		pub fn remove_env(key: &str);
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
		#[wasm_bindgen(js_name = "test_read_dir")]
		pub fn read_dir(path: &str) -> js_sys::Array;
		#[wasm_bindgen(js_name = "test_exists")]
		pub fn exists(path: &str) -> bool;
		#[wasm_bindgen(js_name = "test_create_dir_all")]
		pub fn create_dir_all(path: &str);
		#[wasm_bindgen(js_name = "test_write_file")]
		pub fn write_file(path: &str, content: &[u8]) -> Option<String>;
		#[wasm_bindgen(js_name = "test_remove")]
		pub fn remove(path: &str) -> Option<String>;
		#[wasm_bindgen(js_name = "test_env_args")]
		pub fn env_args() -> js_sys::Array;
		#[wasm_bindgen(js_name = "test_env_var")]
		pub fn env_var(key: &str) -> Option<String>;
		#[wasm_bindgen(js_name = "test_set_env")]
		pub fn set_env(key: &str, value: &str);
		#[wasm_bindgen(js_name = "test_remove_env")]
		pub fn remove_env(key: &str);
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

/// The host JavaScript runtime the wasm module is executing in.
///
/// Unlike the `#[cfg(target_arch = "wasm32")]` split (which is compile-time:
/// "am I wasm bytecode?"), this is a runtime decision: the same wasm binary boots
/// under the Deno test runner, in a browser tab, or under another JS host, and
/// branches here. The standalone `beet` binary uses it to choose its entry: a host
/// with a filesystem (Deno/Node) loads `--main` through `fs_ext`, a `Browser` reads
/// its program from the DOM instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsEnvironment {
	/// The Deno runtime, ie the beet wasm test runner. Has fs/env globals.
	Deno,
	/// Node.js (`process.versions.node`). Has fs/env globals.
	Node,
	/// An embedded engine with no DOM and no process host (eg QuickJs), the
	/// catch-all when no other marker is present.
	QuickJs,
	/// A browser tab: a `window` carrying a `document`. No process / fs.
	Browser,
}

impl JsEnvironment {
	/// Decide the runtime from the presence of host markers. Pure, so the full
	/// matrix is unit-testable off-host.
	///
	/// Ordering is load-bearing: the beet Deno runner polyfills a `window`, so a
	/// `window` alone does not mean browser; `Deno` is therefore checked first and
	/// `Browser` additionally requires a `document` (which Deno does not provide).
	fn detect(
		has_deno: bool,
		has_node: bool,
		has_window: bool,
		has_document: bool,
	) -> Self {
		if has_deno {
			JsEnvironment::Deno
		} else if has_window && has_document {
			JsEnvironment::Browser
		} else if has_node {
			JsEnvironment::Node
		} else {
			JsEnvironment::QuickJs
		}
	}

	/// Whether this runtime has a filesystem reachable through the runner's fs
	/// globals (so `--main` + an `FsStore` entry load), ie Deno or Node. A
	/// `Browser`/`QuickJs` has none, degrading every `fs_ext` call to a no-op.
	pub fn has_fs(&self) -> bool {
		matches!(self, JsEnvironment::Deno | JsEnvironment::Node)
	}
}

/// Whether the host defines a (non-function) global of the given name, eg the
/// `Deno`/`process`/`document` runtime markers. Unlike [`has_global`] this probes
/// the bare name (the markers are real host globals, not the runner's prefixed fs
/// shims) and accepts any defined value, not only functions.
fn global_defined(name: &str) -> bool {
	js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str(name))
		.map(|value| !value.is_undefined() && !value.is_null())
		.unwrap_or(false)
}

/// Whether `process.versions.node` is a string, ie the host is Node.js.
fn has_node_marker() -> bool {
	js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("process"))
		.ok()
		.filter(|process| !process.is_undefined() && !process.is_null())
		.and_then(|process| {
			js_sys::Reflect::get(&process, &JsValue::from_str("versions")).ok()
		})
		.and_then(|versions| {
			js_sys::Reflect::get(&versions, &JsValue::from_str("node")).ok()
		})
		.map(|node| node.is_string())
		.unwrap_or(false)
}

/// Detect the host JavaScript runtime by probing its globals; see [`JsEnvironment`].
pub fn environment() -> JsEnvironment {
	let window = web_sys::window();
	let has_document = window.and_then(|win| win.document()).is_some();
	JsEnvironment::detect(
		global_defined("Deno"),
		has_node_marker(),
		web_sys::window().is_some(),
		has_document,
	)
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

/// List the files under a directory recursively, returning each path relative to
/// `path` (forward-slash separated), ie a `Deno.readDirSync` walk. Empty where the
/// fs global is absent or the directory does not exist. This backs `ReadDir`'s wasm
/// directory walk (see `read_dir.rs`), so an `FsStore` lists the same files on wasm
/// as native.
pub fn read_dir(path: &str) -> Vec<SmolStr> {
	if has_global("read_dir") {
		js_strings(&raw::read_dir(path))
	} else {
		Vec::new()
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

/// Recursively remove a file or directory, ie `Deno.removeSync(.., recursive)`
/// (a missing path errors, like `std::fs::remove_*`). A no-op where the fs global
/// is absent.
pub fn remove(path: &str) -> FsResult {
	match has_global("remove").then(|| raw::remove(path)).flatten() {
		Some(err) => Err(FsError::other(path, err)),
		None => Ok(()),
	}
}

/// The wasm equivalent of process argv (excluding the program name): the deno
/// runner's `Deno.args` when present, else the browser location's path + query as
/// CLI args ([`search_params_ext::location_args`]), else empty (a Worker has
/// neither). The global check comes first since deno also polyfills a `window`, so a
/// deno run must not fall to the browser path.
///
/// This is the single wasm arg decision; [`env_ext::args`](crate::prelude::env_ext)
/// delegates here rather than branching itself.
pub fn args() -> Vec<String> {
	if has_global("env_args") {
		return js_strings(&raw::env_args())
			.into_iter()
			.map(Into::into)
			.collect();
	}
	if environment() == JsEnvironment::Browser {
		return search_params_ext::location_args();
	}
	Vec::new()
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

/// Remove an environment variable, ie `Deno.env.delete(key)`. A no-op where
/// unavailable.
pub fn remove_env(key: &str) {
	if has_global("remove_env") {
		raw::remove_env(key);
	}
}

/// All environment variables as native `(key, value)` pairs, ie
/// `Object.entries(Deno.env.toObject())`. Empty where unavailable.
///
/// Parses the host's 2D JS entries array here (skipping any malformed pair), so
/// callers receive native types rather than a `js_sys::Array` to walk.
pub fn env_all() -> Vec<(SmolStr, SmolStr)> {
	if !has_global("env_all") {
		return Vec::new();
	}
	let entries = raw::env_all();
	(0..entries.length())
		.filter_map(|i| {
			let pair = js_sys::Array::from(&entries.get(i));
			Some((
				SmolStr::from(pair.get(0).as_string()?),
				SmolStr::from(pair.get(1).as_string()?),
			))
		})
		.collect()
}

/// Collect a JS array of strings into native [`SmolStr`]s, skipping any element
/// that is not a string.
fn js_strings(array: &js_sys::Array) -> Vec<SmolStr> {
	(0..array.length())
		.filter_map(|i| array.get(i).as_string())
		.map(SmolStr::from)
		.collect()
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

#[cfg(test)]
mod test {
	use super::*;

	// the marker matrix: Deno wins even when it polyfills `window`; Browser needs
	// both `window` and `document`; Node is the `process` host without a DOM;
	// QuickJs is the marker-less catch-all.
	#[crate::test]
	fn detect_matrix() {
		// (deno, node, window, document) -> env
		JsEnvironment::detect(true, false, false, false)
			.xpect_eq(JsEnvironment::Deno);
		// deno polyfills window: still Deno, never Browser.
		JsEnvironment::detect(true, false, true, false)
			.xpect_eq(JsEnvironment::Deno);
		JsEnvironment::detect(false, false, true, true)
			.xpect_eq(JsEnvironment::Browser);
		// window without a document is not a browser.
		JsEnvironment::detect(false, false, true, false)
			.xpect_eq(JsEnvironment::QuickJs);
		JsEnvironment::detect(false, true, false, false)
			.xpect_eq(JsEnvironment::Node);
		JsEnvironment::detect(false, false, false, false)
			.xpect_eq(JsEnvironment::QuickJs);
		JsEnvironment::has_fs(&JsEnvironment::Deno).xpect_true();
		JsEnvironment::has_fs(&JsEnvironment::Browser).xpect_false();
	}

	// the wasm test harness runs under Deno, so live detection resolves to Deno
	// (a real `Deno` global), exercising the global-probing path end to end.
	#[crate::test]
	fn detects_the_deno_runner() {
		environment().xpect_eq(JsEnvironment::Deno);
	}
}
