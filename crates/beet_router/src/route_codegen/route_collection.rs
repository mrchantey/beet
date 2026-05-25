//! Route collection scanning.
//!
//! A [`RouteCollection`] points at a source directory of route files and is
//! scanned via a [`BlobStore`] into a list of [`RouteFile`]. The same scan is
//! used at codegen time (to emit bundles + typed links) and could be reused at
//! runtime (to spawn `route(path, BlobScene::new(path))` for content files).

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use proc_macro2::Span;
use std::str::FromStr;
use syn::Ident;
use syn::ItemFn;
use syn::Visibility;

/// Categorizes a [`RouteCollection`] by purpose.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum RouteCollectionCategory {
	/// Files expose public functions named after HTTP methods, returning scene
	/// content. Included in the typed route tree.
	#[default]
	Pages,
	/// Files expose server-action handlers returning a serializable type.
	/// Excluded from the route tree and gated behind the `server` feature.
	Actions,
}

impl RouteCollectionCategory {
	/// Whether routes in this category appear in the typed route tree.
	pub fn include_in_route_tree(&self) -> bool {
		matches!(self, Self::Pages)
	}
}

/// A directory of route files scanned into a collection bundle and typed links.
#[derive(Debug, Clone, Component)]
pub struct RouteCollection {
	/// Source directory containing the route files.
	pub src: AbsPathBuf,
	/// Include/exclude filters for the files in [`Self::src`].
	pub filter: GlobFilter,
	/// Whether this is a pages or actions collection.
	pub category: RouteCollectionCategory,
	/// Route prefix prepended to every route in this collection, ie `docs`.
	pub base_route: RelPath,
	/// Cargo feature gating server-only handlers (actions collections).
	///
	/// Defaults to `Some("server")`, matching the convention that server
	/// handlers are excluded from wasm client builds. Set to `None` to emit
	/// the handlers unconditionally.
	pub server_feature: Option<String>,
	/// The generated bundle file for this collection.
	pub codegen: CodegenFile,
}

impl RouteCollection {
	/// Creates a new collection from a source directory and its codegen target.
	///
	/// The default filter excludes `mod.rs` and any `codegen` directory.
	pub fn new(src: AbsPathBuf, codegen: CodegenFile) -> Self {
		Self {
			src,
			filter: GlobFilter::default()
				.with_exclude("*mod.rs")
				.with_exclude("*/codegen/*"),
			category: RouteCollectionCategory::default(),
			base_route: RelPath::default(),
			server_feature: Some("server".into()),
			codegen,
		}
	}

	/// Sets the cargo feature gating server-only handlers, or `None` to emit
	/// them unconditionally.
	pub fn with_server_feature(
		mut self,
		feature: Option<impl Into<String>>,
	) -> Self {
		self.server_feature = feature.map(Into::into);
		self
	}

	/// Sets the category for this collection.
	pub fn with_category(mut self, category: RouteCollectionCategory) -> Self {
		self.category = category;
		self
	}

	/// Sets the glob filter for this collection.
	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	/// Sets the base route prepended to every route in this collection.
	pub fn with_base_route(mut self, base_route: impl Into<RelPath>) -> Self {
		self.base_route = base_route.into();
		self
	}

	/// The snake_case name of this collection, derived from its codegen file.
	pub fn name(&self) -> Result<String> { self.codegen.name() }

	/// A [`BlobStore`] over the source directory, used for scanning.
	pub fn store(&self) -> BlobStore {
		BlobStore::new(FsStore::new(self.src.clone()))
	}

	/// Scans the source directory into a sorted list of [`RouteFile`].
	pub async fn scan(&self) -> Result<Vec<RouteFile>> {
		let store = self.store();
		let mut paths = store.list().await?;
		paths.sort();

		let mut files = Vec::new();
		for store_path in paths {
			if !self.filter.passes(&store_path) {
				continue;
			}
			let Some(ext) = store_path.extension().and_then(|e| e.to_str())
			else {
				continue;
			};
			let route_path =
				self.base_route.join(route_path_from_file(&store_path));
			match ext {
				"rs" => {
					let bytes = store.get(&store_path).await?;
					let src = String::from_utf8(bytes.to_vec())?;
					let methods = parse_route_methods(&src)?;
					if methods.is_empty() {
						continue;
					}
					files.push(RouteFile {
						route_path,
						kind: RouteFileKind::Rust {
							mod_ident: mod_ident_from_file(&store_path),
							mod_path: self.mod_path(&store_path)?,
							methods,
						},
					});
				}
				"md" | "mdx" | "markdown" | "html" => {
					files.push(RouteFile {
						route_path,
						kind: RouteFileKind::Blob {
							store_path: store_path.clone(),
						},
					});
				}
				_ => {}
			}
		}
		Ok(files)
	}

	/// The `#[path = ..]` value for a source file, relative to the codegen
	/// output directory, using forward slashes.
	fn mod_path(&self, store_path: &RelPath) -> Result<String> {
		let abs = self.src.join(store_path.to_string());
		let rel = path_ext::create_relative(self.codegen.output_dir()?, &abs)?;
		Ok(path_ext::to_forward_slash(rel).to_string_lossy().to_string())
	}
}

/// A single route file discovered during a [`RouteCollection::scan`].
#[derive(Debug, Clone)]
pub struct RouteFile {
	/// The full route path (including the collection base route).
	pub route_path: RelPath,
	/// The kind of route file.
	pub kind: RouteFileKind,
}

/// The kind of a [`RouteFile`].
#[derive(Debug, Clone)]
pub enum RouteFileKind {
	/// A Rust handler file exposing one or more HTTP-method functions.
	Rust {
		/// Module identifier for the generated `mod` declaration.
		mod_ident: Ident,
		/// The `#[path = ..]` value for the generated `mod` declaration.
		mod_path: String,
		/// The HTTP handlers in this file.
		methods: Vec<RouteMethod>,
	},
	/// A content file (markdown/html) served via [`BlobScene`].
	Blob {
		/// Path of the content file relative to the store root.
		store_path: RelPath,
	},
}

impl RouteFile {
	/// The methods in this file if it is a Rust handler file.
	pub fn methods(&self) -> &[RouteMethod] {
		match &self.kind {
			RouteFileKind::Rust { methods, .. } => methods,
			RouteFileKind::Blob { .. } => &[],
		}
	}
}

/// An HTTP handler function extracted from a Rust route file.
#[derive(Debug, Clone)]
pub struct RouteMethod {
	/// The HTTP method, matching the function name.
	pub method: HttpMethod,
	/// The parsed handler function.
	pub item: ItemFn,
}

/// Derives the route path from a source file path, stripping the extension and
/// collapsing `index` files into their parent directory.
fn route_path_from_file(store_path: &RelPath) -> RelPath {
	let mut segments: Vec<String> =
		store_path.segments().iter().map(|s| s.to_string()).collect();
	if let Some(last) = segments.last_mut() {
		if let Some(dot) = last.rfind('.') {
			last.truncate(dot);
		}
	}
	if segments.last().map(|s| s == "index").unwrap_or(false) {
		segments.pop();
	}
	RelPath::from_segments(&segments)
}

/// Derives a unique module identifier from a source file path.
fn mod_ident_from_file(store_path: &RelPath) -> Ident {
	let mut segments: Vec<String> =
		store_path.segments().iter().map(|s| s.to_string()).collect();
	if let Some(last) = segments.last_mut() {
		if let Some(dot) = last.rfind('.') {
			last.truncate(dot);
		}
	}
	let joined = segments.join("_");
	Ident::new(&path_to_ident(&joined), Span::call_site())
}

/// Parses a Rust source file into its public HTTP-method handlers.
fn parse_route_methods(src: &str) -> Result<Vec<RouteMethod>> {
	let mut methods = Vec::new();
	for item in syn::parse_file(src)?.items {
		let syn::Item::Fn(func) = item else { continue };
		if !matches!(func.vis, Visibility::Public(_)) {
			continue;
		}
		let Ok(method) = HttpMethod::from_str(&func.sig.ident.to_string())
		else {
			continue;
		};
		methods.push(RouteMethod { method, item: func });
	}
	Ok(methods)
}

/// Converts a path-like string into a valid, deduplicated Rust identifier.
fn path_to_ident(path: &str) -> String {
	let mut ident = String::new();
	let mut chars = path.chars();
	if let Some(first) = chars.next() {
		if first.is_ascii_alphabetic() || first == '_' {
			ident.push(first);
		} else {
			ident.push('_');
			if first.is_ascii_digit() {
				ident.push(first);
			}
		}
	}
	for ch in chars {
		if ch.is_ascii_alphanumeric() || ch == '_' {
			ident.push(ch);
		} else {
			ident.push('_');
		}
	}
	if ident.is_empty() {
		"index".to_string()
	} else if ident == "_" {
		"_index".to_string()
	} else {
		ident.replace("__", "_")
	}
}
