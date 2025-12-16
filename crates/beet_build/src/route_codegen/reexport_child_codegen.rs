use crate::prelude::*;
use beet_core::prelude::*;
use syn::ItemMod;

/// Add a `pub mod #name;` for any child [`CodegenFile`] of a [`CodegenFile`]
pub fn reexport_child_codegen(
	mut query: ParamSet<(
		Populated<(Entity, &Children), Added<CodegenFile>>,
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
							.map(|c| {
								(
									c.name(),
									c.pkg_name().cloned(),
									c.output().clone(),
								)
							})
							.ok()
					})
					.collect::<Vec<_>>(),
			)
		})
		.collect::<Vec<_>>();
	for (entity, child_paths) in items {
		let mut p1 = query.p1();
		let mut codegen = p1.get_mut(entity)?;
		trace!("reexporting child codegen: {}", codegen.output());

		for (child_name, child_pkg_name, child_path) in child_paths {
			if child_pkg_name.as_ref() != codegen.pkg_name() {
				// this is imported from another package dont reexport
				continue;
			}

			let relative_path =
				path_ext::create_relative(codegen.output_dir()?, &child_path)?;
			let relative_path = relative_path.to_string_lossy();
			let name = quote::format_ident!("{child_name}");
			codegen.add_item::<ItemMod>(syn::parse_quote! {
				#[allow(unused, missing_docs)]
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
