use crate::prelude::*;
use beet_utils::prelude::PathExt;
use bevy::prelude::*;
use syn::ItemMod;

/// Add a `pub mod #name;` for any child [`CodegenFile`] of a [`CodegenFile`]
pub fn reexport_child_codegen(
	mut query: ParamSet<(
		Populated<(Entity, &Children), Changed<CodegenFile>>,
		Query<&mut CodegenFile>,
	)>,
) -> Result {
	let items = query
		.p0()
		.iter()
		.map(|(entity, children)| (entity, children.iter().collect::<Vec<_>>()))
		.collect::<Vec<_>>();
	let items = items
		.into_iter()
		.map(|(entity, children)| {
			(
				entity,
				children
					.into_iter()
					.filter_map(|id| {
						query
							.p1()
							.get(id)
							.map(|c| (c.name(), c.output.clone()))
							.ok()
					})
					.collect::<Vec<_>>(),
			)
		})
		.collect::<Vec<_>>();
	for (entity, child_paths) in items {
		let mut p1 = query.p1();
		let mut codegen = p1.get_mut(entity)?;

		for (child_name, child_path) in child_paths {
			let relative_path =
				PathExt::create_relative(codegen.output_dir()?, &child_path)?;
			let relative_path = relative_path.to_string_lossy();
			let name = quote::format_ident!("{child_name}");
			codegen.add_item::<ItemMod>(syn::parse_quote! {
				#[path = #relative_path]
				pub mod #name;
			});
			// codegen.add_item::<ItemUse>(syn::parse_quote! {
			// 	pub use self::#name::*;
			// });
		}
	}
	Ok(())
}
