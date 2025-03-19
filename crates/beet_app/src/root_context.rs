/// Collection of cargo environment variables
/// and file paths required to run a beet app.
///
/// This should be created via the `root_cx!` macro.
///
/// ## example
///
/// ```rust
/// # use beet_app::prelude::*;
///
/// let app = BeetApp::new(root_cx!());
///
/// ```
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RootContext {
	pub file: &'static str,
	pub manifest_path: String,
	pub pkg_version: String,
	pub pkg_name: String,
	pub pkg_description: String,
	pub pkg_homepage: String,
}

impl RootContext {}

/// Create a `RootContext` struct using local file and env macros.
#[macro_export]
macro_rules! root_cx {
	() => {
		RootContext {
			file: file!(),
			manifest_path: env!("CARGO_MANIFEST_PATH").to_string(),
			pkg_version: env!("CARGO_PKG_VERSION").to_string(),
			pkg_name: env!("CARGO_PKG_NAME").to_string(),
			pkg_description: env!("CARGO_PKG_DESCRIPTION").to_string(),
			pkg_homepage: env!("CARGO_PKG_HOMEPAGE").to_string(),
		}
	};
}


// cargo env vars https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let cx = root_cx!();
		expect(cx.file.is_empty()).to_be_false();
		expect(cx.manifest_path.is_empty()).to_be_false();
		expect(cx.pkg_version.is_empty()).to_be_false();
		expect(cx.pkg_name.is_empty()).to_be_false();
		expect(cx.pkg_description.is_empty()).to_be_false();
		expect(cx.pkg_homepage.is_empty()).to_be_false();
	}
}
