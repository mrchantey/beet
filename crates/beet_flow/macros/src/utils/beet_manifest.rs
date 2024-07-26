//! # Beet Mainfest
//! copied from bevy/crates/bevy_macro_utils/src/bevy_manifest.rs
//!
//!

extern crate proc_macro;

use proc_macro::TokenStream;
use std::env;
use std::path::PathBuf;
use toml_edit::DocumentMut;
use toml_edit::Item;

/// The path to the `Cargo.toml` file for the Beet project.
pub struct BeetManifest {
	manifest: DocumentMut,
}

impl Default for BeetManifest {
	fn default() -> Self {
		Self {
			manifest: env::var_os("CARGO_MANIFEST_DIR")
				.map(PathBuf::from)
				.map(|mut path| {
					path.push("Cargo.toml");
					if !path.exists() {
						panic!(
							"No Cargo manifest found for crate. Expected: {}",
							path.display()
						);
					}
					let manifest = std::fs::read_to_string(path.clone())
						.unwrap_or_else(|_| {
							panic!(
								"Unable to read cargo manifest: {}",
								path.display()
							)
						});
					manifest.parse::<DocumentMut>().unwrap_or_else(|_| {
						panic!(
							"Failed to parse cargo manifest: {}",
							path.display()
						)
					})
				})
				.expect("CARGO_MANIFEST_DIR is not defined."),
		}
	}
}
const BEET: &str = "beet";
const PREFIX: &str = "beet_";
// const BEET_INTERNAL: &str = "bevy_internal";

impl BeetManifest {
	/// Attempt to retrieve the [path](syn::Path) of a particular package in
	/// the [manifest](BevyManifest) by [name](str).
	pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
		fn dep_package(dep: &Item) -> Option<&str> {
			if dep.as_str().is_some() {
				None
			} else {
				dep.get("package").map(|name| name.as_str().unwrap())
			}
		}

		let find_in_deps = |deps: &Item| -> Option<syn::Path> {
			if let Some(dep) = deps.get(name) {
				Some(Self::parse_str(dep_package(dep).unwrap_or(name)))
			} else if let Some(dep) = deps.get(BEET) {
				let package = dep_package(dep).unwrap_or(BEET);
				// } else if let Some(dep) = deps.get(BEET_INTERNAL) {
				// 	dep_package(dep).unwrap_or(BEET_INTERNAL)
				let mut path = Self::parse_str::<syn::Path>(package);
				if let Some(module) = name.strip_prefix(PREFIX) {
					path.segments.push(Self::parse_str(module));
				}
				Some(path)
			} else {
				None
			}
		};

		let deps = self.manifest.get("dependencies");
		let deps_dev = self.manifest.get("dev-dependencies");

		deps.and_then(find_in_deps)
			.or_else(|| deps_dev.and_then(find_in_deps))
	}
	pub fn get_path_direct(name: &str) -> syn::Path {
		Self::default().get_path(name)
	}


	/// Returns the path for the crate with the given name.
	pub fn get_path(&self, name: &str) -> syn::Path {
		self.maybe_get_path(name)
			.unwrap_or_else(|| Self::parse_str(name))
	}

	/// Attempt to parse the provided [path](str) as a [syntax tree node](syn::parse::Parse)
	pub fn try_parse_str<T: syn::parse::Parse>(path: &str) -> Option<T> {
		syn::parse(path.parse::<TokenStream>().ok()?).ok()
	}

	/// Attempt to parse provided [path](str) as a [syntax tree node](syn::parse::Parse).
	///
	/// # Panics
	///
	/// Will panic if the path is not able to be parsed. For a non-panicking option, see [`try_parse_str`]
	///
	/// [`try_parse_str`]: Self::try_parse_str
	pub fn parse_str<T: syn::parse::Parse>(path: &str) -> T {
		Self::try_parse_str(path).unwrap()
	}
}
