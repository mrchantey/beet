//! Combinator route tokenization for Markdown and RSX files.
//!
//! This module handles converting parsed combinator tokens (from `.md` and `.rsx` files)
//! into generated Rust code that can be used as route handlers.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use syn::ItemFn;



/// Marker component for route files that have been parsed into a tree via
/// [`CombinatorTokens`], ie `.md` and `.rsx` files.
///
/// This component indicates that the associated [`CodegenFile`] should be
/// populated with a generated `get()` function containing the tokenized RSX.
#[derive(Debug, Clone, Component)]
pub(crate) struct CombinatorRouteCodegen;

/// Tokenizes combinator routes and appends them to their [`CodegenFile`].
///
/// After a [`CombinatorTokens`] has been parsed into a tree, this system:
/// 1. Finds the [`StaticRoot`] child of the route file
/// 2. Temporarily converts it to an [`InstanceRoot`] for tokenization
/// 3. Generates a `pub fn get() -> impl IntoHtml` function
/// 4. Appends the function to the [`CodegenFile`]
pub(crate) fn tokenize_combinator_route(world: &mut World) -> Result {
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
		let tokens = tokenize_rsx_resolve_snippet(world, static_root)?;
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

	#[test]
	fn works() {
		let mut world = BuildPlugin::world();

		world.spawn(RouteFileCollection::test_site_docs());

		world.run_schedule(ParseSourceFiles);
		world
			.query_filtered_once::<&CodegenFile, With<CombinatorRouteCodegen>>(
			)[0]
		.build_output()
		.unwrap()
		.to_token_stream()
		.xpect_snapshot();
	}
}
