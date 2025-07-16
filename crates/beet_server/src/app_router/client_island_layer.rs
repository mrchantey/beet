use crate::prelude::*;
use beet_core::exports::ron;
use beet_core::prelude::*;
use beet_router::prelude::*;
use beet_rsx::as_beet::*;
use bevy::prelude::*;

/// For routes that output a [`BoxedBundle`],
/// return client islands as a string of RON serialized data.
/// These routes will be nested under `route_path`, which defaults to `"/__client_islands"`.
#[template]
pub fn ClientIslandLayer(
	// Place client islands at a specific route path to avoid collisions with the html route
	#[field(default = (ClientIslandLayer::default_route_prefix().into()))]
	route_path: String,
) -> impl Bundle {
	(
		RouteSegment::Static(route_path),
		RouteLayer::after_route(|| {
			client_island_layer
				.run_if(resource_exists::<RouteHandlerOutput<BoxedBundle>>)
		}),
	)
}

impl ClientIslandLayer {
	pub fn default_route_prefix() -> &'static str { "__client_islands" }
}

impl Default for ClientIslandLayer {
	fn default() -> Self {
		Self {
			route_path: Self::default_route_prefix().into(),
		}
	}
}

// take the [`BoxedBundle`] and collect client islands from it
fn client_island_layer(world: &mut World) -> Result {
	let output = world.remove_resource::<RouteHandlerOutput<BoxedBundle>>()
		.unwrap(/*checked*/);
	let entity = output.0.add_to_world(world);
	world.entity_mut(entity).insert(HtmlDocument);
	world.run_schedule(Update);
	let islands =
		world.run_system_cached_with(collect_client_islands, entity)?;
	let islands = ron::ser::to_string(&islands)?;
	world.insert_resource(islands.into_response());
	Ok(())
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::as_beet::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	#[template]
	#[derive(Serialize, Deserialize)]
	pub fn ClientTemplate(foo: u32) -> impl Bundle {
		rsx! {<div>foo: {foo}</div>}
	}

	#[template]
	pub fn BaseTemplate() -> impl Bundle {
		rsx! {
			<ClientTemplate foo=42 client:load/>
		}
	}

	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world.spawn((
			ClientIslandLayer::default().into_node_bundle(),
			RouteInfo::get("/"),
			RouteHandler::new_bundle(|| {
				rsx! {
					<BaseTemplate/>
				}
			}),
		));

		BeetRouter::route_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("[(template:(type_name:\"beet_server::app_router::client_island_layer::test::ClientTemplate\",ron:\"(foo:42)\"),mount:false,dom_idx:(0))]");
	}
}
