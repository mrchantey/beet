use crate::prelude::*;
use beet_common::prelude::TempNonSendMarker;
use beet_utils::prelude::PathExt;
use bevy::prelude::*;
use syn::ItemMod;

/// Add a pub mod #name; for each file group to the root codegen file
/// with a matching package name.
pub fn reexport_file_groups(
	_: TempNonSendMarker,
	mut roots: Query<
		(&mut CodegenFileSendit, &Children),
		(With<RouterCodegenRoot>, Without<FileGroupSendit>),
	>,
	file_groups: Query<(&CodegenFileSendit, &FileGroupSendit)>,
) -> Result {
	for (mut codegen, children) in roots.iter_mut() {
		let codegen_name = codegen.pkg_name.clone();
		for (child_codegen, child_group) in children
			.iter()
			.filter_map(|id| file_groups.get(id).ok())
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
				child_group
					.name
					.as_ref()
					.map(|name| name.as_str())
					.unwrap_or_else(|| {
						child_codegen
							.output
							.file_stem()
							.and_then(|s| s.to_str())
							.unwrap_or("file_group")
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
