use crate::prelude::*;
use beet_utils::prelude::PathExt;
use bevy::prelude::*;
use syn::ItemMod;

/// Add a pub mod #name; for each collection to the root codegen file
/// with a matching package name.
pub fn reexport_collections(
	mut roots: Query<
		(&mut CodegenFile, &Children),
		(With<RouteCodegenRoot>, Without<RouteFileCollection>),
	>,
	collections: Query<(&CodegenFile, &RouteFileCollection)>,
) -> Result {
	for (mut codegen, children) in roots.iter_mut() {
		let codegen_name = codegen.pkg_name.clone();
		for (child_codegen, child_collection) in children
			.iter()
			.filter_map(|id| collections.get(id).ok())
			.filter(|(child_codegen, _)| {
				&child_codegen.pkg_name == &codegen_name
			}) {
			let relative_path = PathExt::create_relative(
				codegen.output_dir()?,
				&child_codegen.output,
			)?;
			let relative_path = relative_path.to_string_lossy();
			let name = quote::format_ident!(
				"{}",
				child_collection
					.name
					.as_ref()
					.map(|name| name.as_str())
					.unwrap_or_else(|| {
						child_codegen
							.output
							.file_stem()
							.and_then(|s| s.to_str())
							.unwrap_or("collection")
					})
			);
			codegen.add_item::<ItemMod>(syn::parse_quote! {
				#[path = #relative_path]
				pub mod #name;
			});
			// codegen.add_item::<ItemUse>(syn::parse_quote! {
			// 	pub use #name;
			// });
		}
	}
	Ok(())
}
