use std::path::Path;
use std::path::PathBuf;
use sweet::prelude::*;

/// Collection of cargo environment variables
/// and file paths required to run a beet app.
///
/// This should be created via the `app_cx!` macro.
///
/// ## example
///
/// ```rust
/// # use beet_router::prelude::*;
///
/// let app = AppRouter::new(app_cx!());
///
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AppContext {
	/// The entry file for the app.
	pub file: WorkspacePathBuf,
	pub manifest_path: PathBuf,
	pub pkg_version: String,
	pub pkg_name: String,
	pub pkg_description: String,
	pub pkg_homepage: String,
}

impl AppContext {
	/// Use the given `file` field as the root for all relative paths.
	pub fn resolve_path(&self, path: impl AsRef<Path>) -> PathBuf {
		self.file.parent().unwrap().join(path)
	}
}

/// Creates an `AppContext` struct using local file and env macros.
///
/// ## Example
///
/// ```rust
/// # use beet_router::prelude::*;
/// let app = AppRouter::new(app_cx!());
#[macro_export]
macro_rules! app_cx {
	() => {
		AppContext {
			file: beet::exports::WorkspacePathBuf::new(file!()),
			manifest_path: env!("CARGO_MANIFEST_PATH").into(),
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
	use beet_rsx::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let cx = app_cx!();
		expect(cx.file.parent()).to_be_some();
		expect(cx.manifest_path.parent()).to_be_some();
		expect(cx.pkg_version.is_empty()).to_be_false();
		expect(cx.pkg_name.is_empty()).to_be_false();
		expect(cx.pkg_description.is_empty()).to_be_false();
		expect(cx.pkg_homepage.is_empty()).to_be_false();
	}
}
