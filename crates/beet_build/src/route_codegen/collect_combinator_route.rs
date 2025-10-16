use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use syn::ItemFn;



/// Added to the root of route files that have been parsed into a tree via
/// [`CombinatorTokens`], ie `.md` and `.rsx` files.
#[derive(Debug, Clone, Component)]
pub struct CombinatorRouteCodegen;

/// After a [`CombinatorTokens`] has been parsed into a [`Bundle`],
/// tokenize it and append to the [`CodegenFile`].
pub fn tokenize_combinator_route(world: &mut World) -> Result {
	let mut query = world
		.query_filtered::<(Entity,&ChildOf), (With<CodegenFile>, Changed<CombinatorRouteCodegen>)>(
		);
	for (entity, parent) in query
		.iter(world)
		.map(|(entity, parent)| (entity, parent.parent()))
		.collect::<Vec<_>>()
	{
		let Some(static_root) = world
			.entity(parent)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.find(|child| world.entity(*child).contains::<StaticRoot>())
			})
			.flatten()
		else {
			bevybail!(
				"CombinatorRouteCodegen has no StaticRoot child: {entity:?}"
			);
		};

		// this is a static but we need an instance, the only difference being
		// StaticRoot vs InstanceRoot
		world
			.entity_mut(static_root)
			.remove::<StaticRoot>()
			.insert(InstanceRoot);
		let tokens = tokenize_bundle_resolve_snippet(world, static_root)?;
		world
			.entity_mut(static_root)
			.remove::<InstanceRoot>()
			.insert(StaticRoot);

		trace!("Tokenizing combinator route for entity: {:?}", entity);
		world
			.entity_mut(entity)
			.get_mut::<CodegenFile>()
			.unwrap() // checked in query filter
			.add_item::<ItemFn>(syn::parse_quote!(
				pub fn get() -> impl IntoHtml {
					#tokens
				}
			));
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default())
			.world_mut()
			.spawn(RouteFileCollection::test_site_docs());

		app.update();
		app.world_mut()
			.query_filtered_once::<&CodegenFile, With<CombinatorRouteCodegen>>(
			)[0]
		.build_output()
		.unwrap()
		.to_token_stream()
		.xpect_snapshot();
	}
}
