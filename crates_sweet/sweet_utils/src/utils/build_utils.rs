/// Running `build.rs` files has several nuances, this struct
/// is a helper that exposes some of these.

// runtime env vars: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
// cargo:: output https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script
pub struct BuildUtils;


impl BuildUtils {
	/// a pattern that when matched to a changed file will rerun the build script
	pub fn rerun_if_changed(path: &str) {
		println!("cargo::rerun-if-changed={path}");
	}

	/// `println!` output is suppressed in build scripts,
	/// emitting as a cargo warning is a workaround.
	pub fn print(message: impl std::fmt::Display) {
		println!("cargo::warning={message}");
	}

	pub fn is_wasm() -> bool {
		std::env::var("CARGO_CFG_TARGET_FAMILY")
			.map(|s| s == "wasm")
			.unwrap_or(false)
	}

	// pub fn cfg_flag(condition: &str){

	// }
}
