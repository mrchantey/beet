fn main() {
	println!("cargo::rerun-if-changed=build.rs");
	let out_dir = std::env::var("OUT_DIR").unwrap();
	std::fs::write(
		format!("{out_dir}/types.rs"),
		"pub const BEST_NUMBER: u8 = 7;",
	)
	.unwrap();
	// todo!("generate types here");
}
