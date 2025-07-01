use crate::prelude::*;
use beet_common::prelude::TempNonSendMarker;
use beet_net::prelude::RoutePath;
use beet_utils::prelude::PathExt;
use beet_utils::utils::PipelineTarget;
use bevy::prelude::*;
use proc_macro2::Span;
use std::path::PathBuf;
use syn::Attribute;
use syn::Ident;
use syn::ItemMod;
use syn::parse_quote;


/// A file that belongs to a [`FileGroup`], spawned as its child.
/// The number of child [`RouteFileMethod`] that will be spawned
/// will be either 1 or 0..many, depending on whether the file
/// is a 'single file route':
/// - `foo.md`: 1
/// - `foo.rs`: 0 or more
/// - `foo.rsx: 0 or more
#[derive(Component)]
// Requires non-default SourceFile
pub struct RouteFile {
	/// The index of the file in the group, used for generating unique identifiers.
	pub index: usize,
	/// The local path to the rust file containing the routes.
	/// By default this is the [`SourceFile`] relative to the
	/// [`CodegenFile::output_dir`] but may be modified, for example [`parse_route_file_md`]
	/// will change the path to point to the newly generated `.rs` codegen file.
	pub mod_path: PathBuf,
	/// The [`SourceFile`] relative to [`FileGroup::src`],
	/// Used for per-file codegen.
	pub group_path: PathBuf,
	/// The route path for the file, derived from the file path
	/// relative to the [`FileGroup::src`].
	pub route_path: RoutePath,
}

impl RouteFile {
	/// The identifier for the module import in the generated code.
	pub fn mod_ident(&self) -> syn::Ident {
		Ident::new(&format!("route{}", self.index), Span::call_site())
	}
	/// The module import for the generated code.
	/// For Actions this will only export in non-wasm builds
	pub fn item_mod(&self, category: FileGroupCategory) -> ItemMod {
		let ident = self.mod_ident();
		let path = &self.mod_path.to_string_lossy();
		let target: Option<Attribute> = match category {
			FileGroupCategory::Pages => None,
			FileGroupCategory::Actions => Some(parse_quote! {
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

/// Search the directory of each [`FileGroup`] and parse each file
pub fn spawn_route_files(
	_: TempNonSendMarker,
	mut commands: Commands,
	query: Populated<
		(Entity, &FileGroupSendit, &CodegenFileSendit),
		Added<FileGroupSendit>,
	>,
) -> Result {
	for (entity, group, codegen) in query.iter() {
		let mut entity = commands.entity(entity);
		for (index, abs_path) in group.collect_files()?.into_iter().enumerate()
		{
			let mod_path =
				PathExt::create_relative(&codegen.output_dir()?, &abs_path)?;
			let route_path = PathExt::create_relative(&group.src, &abs_path)?
				.xmap(RoutePath::from_file_path)?;

			let group_path = PathExt::create_relative(&group.src, &abs_path)?;

			entity.with_child((SourceFile::new(abs_path), RouteFile {
				index,
				group_path,
				mod_path,
				route_path,
			}));
		}
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use std::ops::Deref;
	use std::path::PathBuf;

	use crate::prelude::*;
	use beet_utils::utils::PipelineTarget;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let group = world.spawn(FileGroup::test_site_pages()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		let file = world.entity(group).get::<Children>().unwrap()[0];
		let source_file = world.entity(file).get::<SourceFile>().unwrap();
		let route_file = world.entity(file).get::<RouteFile>().unwrap();

		source_file.to_string_lossy().xpect().to_end_with(
			"/beet/crates/beet_router/src/test_site/pages/docs/index.rs",
		);
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
