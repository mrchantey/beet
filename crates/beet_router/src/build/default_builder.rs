use anyhow::Result;

pub struct DefaultBuilder {
	/// The name of the package being built.
	/// By default this is set by `std::env::var("CARGO_PKG_NAME")`
	pkg_name: String,
}


impl Default for DefaultBuilder {
	fn default() -> Self {
		Self {
			pkg_name: std::env::var("CARGO_PKG_NAME")
				.expect("DefaultBuilder: CARGO_PKG_NAME not set"),
		}
	}
}


impl DefaultBuilder {
	pub fn build(self) -> Result<()> { Ok(()) }
}
