use wasm_bindgen::prelude::*;
#[wasm_bindgen]
unsafe extern "C" {
	#[wasm_bindgen]
	/// Get the current working directory, ie `Deno.cwd()`
	/// just like [`std::process::cwd`], this will be relative to the crate, ie
	/// cargo test --workspace is different cwd from cargo test -p my_crate
	pub fn cwd() -> String;
	#[wasm_bindgen]
	/// Use this instead of `std::process::exit` which outputs
	/// an unholy `Uncaught RuntimeError: unreachable`
	pub fn exit(code: i32);
	#[wasm_bindgen(catch)]
	/// Just run the function outside of the wasm boundary
	/// ie `const panic_to_error = (f)=>f()`
	pub fn panic_to_error(
		f: &mut dyn FnMut() -> Result<(), String>,
	) -> Result<(), JsValue>;
	#[wasm_bindgen]
	/// Read a file from the filesystem, ie `Deno.read_file()`
	pub fn read_file(path: &str) -> Option<String>;
	#[wasm_bindgen]
	/// Get the SWEET_ROOT env var, ie `Deno.env.get("SWEET_ROOT")`
	pub fn sweet_root() -> Option<String>;
	#[wasm_bindgen]
	/// Get single environment variable by key, ie `Deno.env.get(key)`
	#[wasm_bindgen(js_name = env_var)]
	pub fn env_var(key: &str) -> Option<String>;
	#[wasm_bindgen]
	/// Get all environment variables as JSON, ie `JSON.stringify(Deno.env.toObject())`
	#[wasm_bindgen(js_name = env_all_json)]
	pub fn env_all_json() -> String;
}



