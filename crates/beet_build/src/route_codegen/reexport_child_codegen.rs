use crate::prelude::*;
use beet_utils::prelude::PathExt;
use bevy::prelude::*;
use heck::ToSnakeCase;
use syn::ItemMod;

/// Add a `pub mod #name;` for any child [`CodegenFile`] of a [`CodegenFile`]
pub fn reexport_child_codegen(
	mut roots: Populated<(Entity, &Children), With<CodegenFile>>,
	mut codegens: Query<&mut CodegenFile>,
) -> Result {
	for (entity, children) in roots.iter_mut() {
		let child_paths = children
			.iter()
			.filter_map(|id| codegens.get(id).map(|c| c.output.clone()).ok())
			.collect::<Vec<_>>();

		let mut codegen = codegens.get_mut(entity)?;
		if !codegen.is_changed() {
			// we cant use Changed<CodegenFile> due to disjoint queries so perform
			// a manual check
			continue;
		}

		for child_path in child_paths {
			let relative_path =
				PathExt::create_relative(codegen.output_dir()?, &child_path)?;
			let relative_path = relative_path.to_string_lossy();
			let name = match child_path
				.file_stem()
				.expect("codegen output must have a file stem")
				.to_str()
				.expect("file stem must be valid UTF-8")
			{
				"mod" => {
					let parent = child_path
						.parent()
						.expect("mod files must have a parent");
					parent
						.file_name()
						.expect("parent must have a file name")
						.to_str()
						.expect("file name must be valid UTF-8")
						.to_snake_case()
				}
				other => other.to_snake_case(),
			};
			let name = quote::format_ident!("{}", name);
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
