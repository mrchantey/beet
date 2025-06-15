use crate::prelude::*;
use beet_common::Sendit;
use beet_common::as_beet::*;
use beet_parse::prelude::tokenize_bundle;
use bevy::prelude::*;
use syn::Block;
use syn::ItemFn;

/// After a [`CombinatorTokens`] has been parsed into a [`Bundle`],
/// tokenize it and append to the [`CodegenFile`].
pub fn combinator_route_codegen(world: &mut World) -> Result {
	let mut query =
		world.query_filtered::<Entity, (
			With<CodegenFileSendit>,
			With<CombinatorRouteCodegenSendit>
		)>();
	for entity in query.iter(world).collect::<Vec<_>>() {
		let tokens = tokenize_bundle(world, entity)?;

		world
			.entity_mut(entity)
			.get_mut::<CodegenFileSendit>()
			.unwrap() // checked in query filter
			.add_item::<ItemFn>(syn::parse_quote!(
				pub fn get()-> impl Bundle{
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
	pub config: Option<Block>,
}

/// insert the config function into the codegen file if it exists
pub fn markdown_route_codegen(
	_: TempNonSendMarker,
	mut query: Populated<
		(&mut CodegenFileSendit, &CombinatorRouteCodegenSendit),
		Added<CombinatorRouteCodegenSendit>,
	>,
) {
	for (mut codegen_file, markdown_codegen) in query.iter_mut() {
		if let Some(config) = &markdown_codegen.config {
			codegen_file.add_item::<ItemFn>(syn::parse_quote!(
				pub fn config_get()-> impl Bundle{
					#config
				}
			));
		}
	}
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
		app.world_mut().spawn(FileGroup::test_site_markdown());
		app.update();
		let codegen = app
			.world_mut()
			.query_filtered_once::<&CodegenFileSendit, With<CombinatorRouteCodegenSendit>>(
			)[0]
		.build_output()
		.unwrap()
		.to_token_stream()
		.to_string();
		expect(&codegen).to_contain("pub fn config_get () -> impl Bundle {");
		expect(&codegen).to_contain("pub fn get () -> impl Bundle {");
	}
}
