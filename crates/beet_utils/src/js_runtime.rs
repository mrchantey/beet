use wasm_bindgen::prelude::*;
#[wasm_bindgen]
unsafe extern "C" {
	/// Get the current working directory, ie `Deno.cwd()`
	/// just like [`std::process::cwd`], this will be relative to the crate, ie
	/// cargo test --workspace is different cwd from cargo test -p my_crate
	#[wasm_bindgen]
	pub fn cwd() -> String;
	/// Use this instead of `std::process::exit` which outputs
	/// an unholy `Uncaught RuntimeError: unreachable`
	#[wasm_bindgen]
	pub fn exit(code: i32);
	/// Just run the function outside of the wasm boundary
	/// ie `const panic_to_error = (f)=>f()`
	#[wasm_bindgen(catch)]
	pub fn panic_to_error(
		f: &mut dyn FnMut() -> Result<(), String>,
	) -> Result<(), JsValue>;
	/// Read a file from the filesystem, ie `Deno.read_file()`
	#[wasm_bindgen]
	pub fn read_file(path: &str) -> Option<String>;
	/// Get the SWEET_ROOT env var, ie `Deno.env.get("SWEET_ROOT")`
	#[wasm_bindgen]
	pub fn sweet_root() -> Option<String>;
	/// Get single environment variable by key, ie `Deno.env.get(key)`
	#[wasm_bindgen]
	pub fn env_var(key: &str) -> Option<String>;
	/// Get all environment variables as entries 2D array, ie `Object.entries(Deno.env.toObject())`
	#[wasm_bindgen]
	pub fn env_all() -> js_sys::Array;
}
