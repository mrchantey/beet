use crate::prelude::*;
use beet_bevy::bevyhow;
use beet_common::Sendit;
use beet_common::as_beet::*;
use beet_parse::prelude::tokenize_bundle;
use bevy::prelude::*;
use syn::Block;
use syn::ItemFn;

/// After a [`CombinatorTokens`] has been parsed into a [`Bundle`],
/// tokenize it and append to the [`CodegenFile`].
pub fn tokenize_combinator_route(world: &mut World) -> Result {
	let mut query =
		world.query_filtered::<Entity, (
			With<CodegenFileSendit>,
			Added<CombinatorRouteCodegenSendit>
		)>();
	for entity in query.iter(world).collect::<Vec<_>>() {
		let tokens = tokenize_bundle(world, entity)?;
		trace!("Tokenizing combinator route for entity: {:?}", entity);
		world
			.entity_mut(entity)
			.get_mut::<CodegenFileSendit>()
			.unwrap() // checked in query filter
			.add_item::<ItemFn>(syn::parse_quote!(
				pub fn get() -> impl Bundle{
					#tokens
				}
			));
	}
	Ok(())
}


/// Added to the root of route files that have been parsed into a tree via
/// [`CombinatorTokens`], ie `.md` and `.rsx` files.
#[derive(Debug, Clone, Sendit)]
#[sendit(derive(Component))]
pub struct CombinatorRouteCodegen {
	pub meta: Option<Block>,
}

/// insert the config function into the codegen file if it exists
pub fn collect_combinator_route(
	_: TempNonSendMarker,
	mut query: Populated<
		(
			Entity,
			&mut CodegenFileSendit,
			&CombinatorRouteCodegenSendit,
		),
		Added<CombinatorRouteCodegenSendit>,
	>,
	parents: Query<&ChildOf>,
	file_groups: Query<&FileGroupSendit>,
) -> Result {
	for (entity, mut codegen_file, markdown_codegen) in query.iter_mut() {
		if let Some(meta) = &markdown_codegen.meta {
			let file_group = parents
				.iter_ancestors(entity)
				.find_map(|e| file_groups.get(e).ok())
				.ok_or_else(|| bevyhow!("failed to find parent FileGroup"))?;
			let meta_type = &file_group.meta_type;
			codegen_file.add_item::<ItemFn>(syn::parse_quote!(
				pub fn meta_get()-> #meta_type{
					#meta.map_err(|err|{
						format!("Failed to parse meta: {}", err)
					}).unwrap()
				}
			));
		}
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::WorldMutExt;
	use beet_parse::prelude::NodeTokensPlugin;
	use bevy::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins((CodegenNativePlugin, NodeTokensPlugin));
		app.world_mut().spawn(FileGroup::test_site_docs());
		app.update();
		let codegen = app
			.world_mut()
			.query_filtered_once::<&CodegenFileSendit, With<CombinatorRouteCodegenSendit>>(
			)[0]
		.build_output()
		.unwrap()
		.to_token_stream()
		.to_string();
		expect(&codegen).to_contain("pub fn meta_get () -> () {");
		expect(&codegen).to_contain("pub fn get () -> impl Bundle {");
	}
}
