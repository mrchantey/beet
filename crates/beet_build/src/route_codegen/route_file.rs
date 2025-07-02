use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_net::prelude::RoutePath;
use beet_utils::prelude::AbsPathBuf;
use beet_utils::prelude::PathExt;
use beet_utils::utils::PipelineTarget;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use proc_macro2::Span;
use std::path::PathBuf;
use syn::Attribute;
use syn::Ident;
use syn::ItemMod;
use syn::parse_quote;


/// A file that belongs to a [`RouteFileCollection`], spawned as its child.
/// The number of child [`RouteFileMethod`] that will be spawned
/// will be either 1 or 0..many, depending on whether the file
/// is a 'single file route':
/// - `foo.md`: 1
/// - `foo.rs`: 0 or more
/// - `foo.rsx: 0 or more
#[derive(Component)]
pub struct RouteFile {
	/// The index of the file in the group, used for generating unique identifiers.
	pub index: usize,
	/// The local path to the rust file containing the routes.
	/// By default this is the [`SourceFile`] relative to the
	/// [`CodegenFile::output_dir`] but may be modified, for example [`parse_route_file_md`]
	/// will change the path to point to the newly generated `.rs` codegen file.
	pub mod_path: PathBuf,
	/// The [`SourceFile`] path, used for efficient match checking
	pub abs_path: AbsPathBuf,
	/// The [`SourceFile`] relative to [`RouteFileCollection::src`],
	/// Used for per-file codegen.
	pub collection_path: PathBuf,
	/// The route path for the file, derived from the file path
	/// relative to the [`RouteFileCollection::src`].
	pub route_path: RoutePath,
}

impl RouteFile {
	/// The identifier for the module import in the generated code.
	pub fn mod_ident(&self) -> syn::Ident {
		Ident::new(&format!("route{}", self.index), Span::call_site())
	}
	/// The module import for the generated code.
	/// For Actions this will only export in non-wasm builds
	pub fn item_mod(&self, category: RouteFileCategory) -> ItemMod {
		let ident = self.mod_ident();
		let path = &self.mod_path.to_string_lossy();
		let target: Option<Attribute> = match category {
			RouteFileCategory::Page => None,
			RouteFileCategory::Action => Some(parse_quote! {
				#[cfg(not(target_arch = "wasm32"))]
			}),
		};

		// currently we use a pub mod for client island resolution,
		// this may change if we go for bevy reflect instead
		syn::parse_quote! {
			#[path = #path]
			#target
			pub mod #ident;
		}
	}
}

#[derive(Default, Resource)]
pub(super) struct CollectionIndexCounter(HashMap<Entity, usize>);

impl CollectionIndexCounter {
	/// Get the next index for the given collection entity,
	/// incrementing the counter for the next call.
	pub fn next(&mut self, entity: Entity) -> usize {
		let index = self.0.entry(entity).or_default();
		let current_index = *index;
		*index += 1;
		current_index
	}
}

/// When a [`FileExprHash`] changes, create a corresponding [`RouteFile`]
/// for each file group that it matches if it doesnt exist,
/// otherwise mark it as changed
pub(super) fn update_route_files(
	mut index_counter: Local<CollectionIndexCounter>,
	mut commands: Commands,
	changed_exprs: Populated<&SourceFile, Changed<FileExprHash>>,
	collections: Query<(Entity, &RouteFileCollection, &CodegenFile)>,
	children: Query<&Children>,
	mut route_files: Query<&mut RouteFile>,
) -> Result {
	for file in changed_exprs.iter() {
		for (collection_entity, collection, codegen) in collections
			.iter()
			.filter(|(_, collection, _)| collection.passes_filter(file))
		{
			if !children.iter_direct_descendants(collection_entity).any(
				|child| match route_files.get_mut(child) {
					Ok(mut route_file) if route_file.abs_path == **file => {
						route_file.set_changed();
						true
					}
					_ => false,
				},
			) {
				// no existing route file found, create a new one
				let mod_path =
					PathExt::create_relative(&codegen.output_dir()?, &file)?;
				let route_path =
					PathExt::create_relative(&collection.src, &file)?
						.xmap(RoutePath::from_file_path)?;

				let collection_path =
					PathExt::create_relative(&collection.src, &file)?;


				let index = index_counter.next(collection_entity);

				commands.spawn((ChildOf(collection_entity), RouteFile {
					index,
					collection_path,
					mod_path,
					route_path,
					abs_path: (**file).clone(),
				}));
			}
		}
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::route_codegen::update_route_files;
	use beet_utils::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use std::ops::Deref;
	use std::path::PathBuf;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::without_fs());

		let index_path = WsPathBuf::new(
			"crates/beet_router/src/test_site/pages/docs/index.rs",
		);

		let group = app
			.world_mut()
			.spawn(RouteFileCollection::test_site_pages())
			.id();
		app.world_mut()
			.spawn(SourceFile::new(index_path.into_abs()));

		app.update();

		app.world_mut()
			.run_system_once(update_route_files)
			.unwrap()
			.unwrap();
		let file = app.world().entity(group).get::<Children>().unwrap()[0];
		let route_file = app.world().entity(file).get::<RouteFile>().unwrap();

		route_file
			.mod_path
			.xref()
			.xpect()
			.to_be(&PathBuf::from("../pages/docs/index.rs"));
		route_file
			.route_path
			.xref()
			.deref()
			.xpect()
			.to_be(&PathBuf::from("/docs"));
	}
}
