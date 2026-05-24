use beet_core::prelude::*;

/// Locations for the `wasm-bindgen` output of a beet client build.
#[derive(Debug, Clone)]
pub struct HtmlConstants {
	/// Directory (relative to the output root) the wasm artifacts are written to.
	pub wasm_dir: RelPath,
	/// The `--out-name` passed to `wasm-bindgen`, ie `main` → `main_bg.wasm`.
	pub wasm_name: String,
}

impl Default for HtmlConstants {
	fn default() -> Self {
		Self {
			wasm_dir: RelPath::new("wasm"),
			wasm_name: "main".to_string(),
		}
	}
}
