/// for example implementation see crates/sweet/cli/src/test_runners/deno.ts
pub mod js_runtime {
	use wasm_bindgen::prelude::*;
	#[wasm_bindgen]
	extern "C" {
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
	}
}


#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod test {
	use crate::prelude::*;

	#[test]
	fn cwd() { expect(js_runtime::cwd()).to_contain("sweet"); }

	#[test]
	#[ignore = "take hook shenanigans"]
	// #[should_panic]
	fn panic_to_error() {
		let result = js_runtime::panic_to_error(&mut || panic!("it panicked"));
		expect(&format!("{:?}", result))
			.to_start_with("Err(JsValue(RuntimeError: unreachable");
	}
	#[test]
	fn read_file() {
		expect(js_runtime::read_file("foobar")).to_be_none();
		expect(js_runtime::read_file("Cargo.toml")).to_be_some();
		// expect(js_runtime::read_file("Cargo.lock")).to_be_some();
	}
	#[test]
	fn sweet_root() {
		expect(js_runtime::sweet_root().unwrap().replace("\\", "/"))
			.to_end_with("beet/");
	}
}
