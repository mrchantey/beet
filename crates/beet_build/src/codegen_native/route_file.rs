use crate::prelude::*;
use beet_common::prelude::TempNonSendMarker;
use beet_utils::prelude::AbsPathBuf;
use beet_utils::prelude::PathExt;
use bevy::prelude::*;
use proc_macro2::Span;
use std::path::PathBuf;
use syn::Ident;
use syn::ItemMod;


/// A file that belongs to a [`FileGroup`], spawned as its child.
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
	/// The path to the original file specified by the [`FileGroup`]. This may be
	/// of any file type.
	pub origin_path: AbsPathBuf,
	/// The local path to the rust file containing the routes.
	/// By default this is the [`origin_path`](Self::origin_path) relative to the
	/// [`FileGroup::src`] but may be modified, for example [`parse_route_file_md`]
	/// will change the path to point to the newly generated `.rs` codegen file.
	pub mod_path: PathBuf,
}

impl RouteFile {
	/// The identifier for the module import in the generated code.
	pub fn mod_ident(&self) -> syn::Ident {
		Ident::new(&format!("route{}", self.index), Span::call_site())
	}
	/// The module import for the generated code.
	pub fn item_mod(&self) -> ItemMod {
		let ident = self.mod_ident();
		let path = &self.mod_path.to_string_lossy();
		syn::parse_quote! {
			#[path = #path]
			mod #ident;
		}
	}
}

/// Search the directory of each [`FileGroup`] and parse each file
pub fn spawn_route_files(
	_: TempNonSendMarker,
	mut commands: Commands,
	query: Populated<(Entity, &FileGroupSendit), Added<FileGroupSendit>>,
) -> Result {
	for (entity, group) in query.iter() {
		let mut entity = commands.entity(entity);
		for (index, abs_path) in group.collect_files()?.into_iter().enumerate()
		{
			let local_path = PathExt::create_relative(&group.src, &abs_path)?;

			entity.with_child(RouteFile {
				index,
				origin_path: abs_path,
				mod_path: local_path,
			});
		}
	}
	Ok(())
}
