use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::RoutePath;
use bevy::prelude::*;
use proc_macro2::Span;
use std::path::PathBuf;
use syn::Attribute;
use syn::Ident;
use syn::ItemMod;
use syn::parse_quote;


/// A file that belongs to a [`RouteFileCollection`], spawned as its child.
/// The number of child [`RouteFileMethod`] depends on the file type:
/// - `foo.md`: 1
/// - `foo.rs`: 0 or more
/// - `foo.rsx`: 0 or more
#[derive(Debug, Component)]
pub struct RouteSourceFile {
	/// The local path to the rust file containing the routes.
	/// By default this is the [`SourceFile`] relative to the
	/// [`CodegenFile::output_dir`] but may be modified with `bypass_change_detection`,
	/// for example [`parse_route_file_md`]
	/// will change the path to point to the newly generated `.rs` codegen file.
	pub mod_path: PathBuf,
	/// The [`SourceFile`] relative to [`RouteFileCollection::src`],
	/// Used for per-file codegen.
	pub source_file_collection_rel: PathBuf,
	/// The route path for the file, derived from the file path
	/// relative to the [`RouteFileCollection::src`].
	pub route_path: RoutePath,
}

impl RouteSourceFile {
	/// The identifier for the module import in the generated code.
	pub fn mod_ident(&self) -> syn::Ident {
		let path = path_to_ident(&self.route_path.to_string_lossy());
		Ident::new(&path, Span::call_site())
	}
	/// The module import for the generated code.
	/// For Actions this will only export in non-wasm builds
	pub fn item_mod(&self, category: RouteCollectionCategory) -> ItemMod {
		let ident = self.mod_ident();
		let path = &self.mod_path.to_string_lossy();
		let cfg: Option<Attribute> = match category {
			RouteCollectionCategory::Pages => None,
			RouteCollectionCategory::Actions => Some(parse_quote! {
				#[cfg(feature = "server")]
			}),
		};

		// currently we use a pub mod for client island resolution,
		// this may change if we go for bevy reflect instead
		syn::parse_quote! {
			#[path = #path]
			#cfg
			pub mod #ident;
		}
	}
}

/// Reset every [`CodegenFile`] ancestor of a changed [`FileExprHash`],
/// includiing both [`RouteFileCollection`] and [`StaticRouteTree`]
pub fn reset_codegen_files(
	changed_exprs: Populated<Entity, Changed<FileExprHash>>,
	mut parent_codegen: Query<&mut CodegenFile>,
	parents: Query<&ChildOf>,
) {
	for file in changed_exprs.iter() {
		for parent in parents.iter_ancestors(file) {
			if let Ok(mut codegen) = parent_codegen.get_mut(parent) {
				trace!("Resetting changed codegen: {}", codegen.output);
				codegen.set_added();
				codegen.clear_items();
			}
		}
	}
}

/// Add a [`RouteSourceFile`] to any newly created [`SourceFile`]
/// that is a child of a [`RouteFileCollection`].
pub(super) fn create_route_files(
	mut commands: Commands,
	query: Populated<
		(Entity, &SourceFile),
		(Added<SourceFile>, Without<RouteSourceFile>),
	>,
	collections: Query<(&RouteFileCollection, &CodegenFile)>,
	parents: Query<&ChildOf>,
) -> Result {
	// sort the items so the index is stable
	let mut items = query.iter().collect::<Vec<_>>();
	items.sort_by_key(|(_, file)| (*file).clone());

	for (entity, file) in items.into_iter() {
		let Some((collection, codegen)) = parents
			.iter_ancestors(entity)
			.find_map(|en| collections.get(en).ok())
		else {
			// this source file is not a descendent of a collection
			continue;
		};

		// no existing route file found, create a new one
		let mod_path = PathExt::create_relative(&codegen.output_dir()?, &file)?;
		let route_path = PathExt::create_relative(&collection.src, &file)?
			.xmap(RoutePath::from_file_path)?;

		let source_file_collection_rel =
			PathExt::create_relative(&collection.src, &file)?;

		debug!("Creating new RouteSourceFile: {}", file.path());

		commands.entity(entity).insert(RouteSourceFile {
			source_file_collection_rel,
			mod_path,
			route_path,
		});
	}
	Ok(())
}

fn path_to_ident(path: &str) -> String {
	let mut ident = String::new();
	let mut chars = path.chars();

	// Handle first character
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use quote::ToTokens;
	use std::ops::Deref;
	use std::path::PathBuf;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default());

		let group = app
			.world_mut()
			.spawn(RouteFileCollection::test_site_pages())
			.id();

		app.update();

		let source_file_entity =
			app.world().entity(group).get::<Children>().unwrap()[0];
		let route_file = app
			.world()
			.entity(source_file_entity)
			.get::<RouteSourceFile>()
			.unwrap();

		route_file
			.mod_path
			.xref()
			.xpect_eq(PathBuf::from("../pages/docs/index.rs"));
		route_file
			.route_path
			.xref()
			.deref()
			.xpect_eq(PathBuf::from("/docs"));

		route_file
			.item_mod(RouteCollectionCategory::Pages)
			.to_token_stream()
			.xpect_snapshot();
	}
}
