use crate::prelude::*;
use wasm_bindgen::prelude::*;

#[cfg(not(test))]
#[wasm_bindgen]
unsafe extern "C" {
	/// Get the current working directory, ie `Deno.cwd()`
	/// just like [`std::process::cwd`], this will be relative to the crate, ie
	/// cargo test --workspace is different cwd from cargo test -p my_crate
	#[wasm_bindgen]
	pub fn cwd() -> String;
	/// Use this instead of `std::process::exit` which outputs
	/// an unholy `Uncaught RuntimeError: unreachable`.
	/// This also propagates the exit code.
	#[wasm_bindgen]
	pub fn exit(code: i32);
	/// Just run the function outside of the wasm boundary
	/// ie `const catch_no_abort_inner = (f)=>f()`
	#[wasm_bindgen(catch)]
	fn catch_no_abort_inner(
		f: &mut dyn FnMut() -> Result<(), String>,
	) -> Result<(), JsValue>;
	/// Read a file from the filesystem, ie `Deno.read_file()`
	#[wasm_bindgen]
	pub fn read_file(path: &str) -> Option<Vec<u8>>;
	/// Ensure a directory exists, ie `Deno.ensureDir()`
	#[wasm_bindgen]
	pub fn create_dir_all(path: &str);
	/// Write a file to the filesystem, ie `Deno.writeTextFileSync()`
	#[wasm_bindgen]
	pub fn write_file(path: &str, content: &[u8]) -> Option<String>;
	/// Get all command line arguments as array, ie `Deno.args`
	#[wasm_bindgen]
	pub fn env_args() -> js_sys::Array;
	/// Get single environment variable by key, ie `Deno.env.get(key)`
	#[wasm_bindgen]
	pub fn env_var(key: &str) -> Option<String>;
	/// Get all environment variables as entries 2D array, ie `Object.entries(Deno.env.toObject())`
	#[wasm_bindgen]
	pub fn env_all() -> js_sys::Array;
}

// TODO this is just to get it to compile, we need a better solution
#[cfg(test)]
#[wasm_bindgen]
unsafe extern "C" {
	#[wasm_bindgen(js_name = "test_cwd")]
	pub fn cwd() -> String;
	#[wasm_bindgen(js_name = "test_exit")]
	pub fn exit(code: i32);
	#[wasm_bindgen(catch, js_name = "test_catch_no_abort_inner")]
	fn catch_no_abort_inner(
		f: &mut dyn FnMut() -> Result<(), String>,
	) -> Result<(), JsValue>;
	#[wasm_bindgen(js_name = "test_read_file")]
	pub fn read_file(path: &str) -> Option<Vec<u8>>;
	#[wasm_bindgen(js_name = "test_create_dir_all")]
	pub fn create_dir_all(path: &str);
	#[wasm_bindgen(js_name = "test_write_file")]
	pub fn write_file(path: &str, content: &[u8]) -> Option<String>;
	#[wasm_bindgen(js_name = "test_env_args")]
	pub fn env_args() -> js_sys::Array;
	#[wasm_bindgen(js_name = "test_env_var")]
	pub fn env_var(key: &str) -> Option<String>;
	#[wasm_bindgen(js_name = "test_env_all")]
	pub fn env_all() -> js_sys::Array;
}

#[deprecated]
pub fn panic_to_error(
	func: impl FnOnce() -> Result<(), String>,
) -> Result<(), JsValue> {
	let mut opt = Some(func);
	catch_no_abort_inner(&mut || opt.take().expect("function already called")())
}


/// Run the function returning an opaque `()` error in the case of a panic.
/// This has a relatively symmetrical api to `panic::catch_unwind`
pub fn catch_no_abort(
	func: impl FnOnce() -> Result<(), String>,
) -> Result<Result<(), String>, ()> {
	let mut opt = Some(func);
	let outcome = catch_no_abort_inner(&mut || {
		opt.take().expect("function already called")()
	});
	match outcome {
		Ok(()) => Ok(Ok(())),
		Err(err) if err.is_string() => {
			crate::cross_log!("twas error!");
			Ok(Err(err.as_string().expect("checked")))
		}
		Err(_) => Err(()),
	}
}
