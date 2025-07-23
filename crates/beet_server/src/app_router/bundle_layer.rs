use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::as_beet::*;
use bevy::prelude::*;

/// Added by the [`AppRouterPlugin`], this layer will convert any
/// [`RouteHandlerOutput<BoxedBundle>`] into a [`HtmlDocument`] and return
/// as [`Html`] response.
/// This excludes any [`BoxedBundle`] consumed in the [`AfterRoute`] step,
/// for example by the [`ClientIslandLayer`].
pub fn bundle_layer(world: &mut World) -> Result {
	let output = world.remove_resource::<RouteHandlerOutput<BoxedBundle>>()
		.unwrap(/*checked*/);
	let entity = output.0.add_to_world(world);
	world.entity_mut(entity).insert(HtmlDocument);
	// println!("Before");
	// world.log_component_names(entity);
	world.run_schedule(Update);
	// println!("After");
	// world.log_component_names(entity);
	let html = world.run_system_cached_with(render_fragment, entity)?;

	world.insert_resource(Html(html).into_response());
	Ok(())
}

/// If the world contains a [`HtmlDocument`] and there is no current [`Response`],
/// convert the document to a response.
pub fn documents_to_response(world: &mut World) -> Result {
	if let Some(&entity) = world
		.query_filtered_once::<Entity, With<HtmlDocument>>()
		.iter()
		.next()
	{
		world.run_schedule(Update);
		let html = world.run_system_cached_with(render_fragment, entity)?;

		world.insert_resource(Html(html).into_response());
	}
	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn creates_resource() {
		let mut world = World::new();
		world = RouteHandler::new_bundle(|| ()).run(world).await;
		world.resource::<RouteHandlerOutput<BoxedBundle>>();

		async fn foo(world: World) -> (World, ()) { (world, ()) }

		let mut world = World::new();
		world = RouteHandler::new_async_bundle(foo).run(world).await;
		world.resource::<RouteHandlerOutput<BoxedBundle>>();
	}


	#[template]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		rsx! {
			<div>foo: {foo}</div>
		}
	}

	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world.spawn((
			RouteInfo::get("/"),
			RouteHandler::new_bundle(|| {
				rsx! {
					<MyTemplate foo=42/>
				}
			}),
		));

		BeetRouter::oneshot_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str(
				"<!DOCTYPE html><html><head></head><body><div>foo: 42</div></body></html>",
			);
	}
	#[sweet::test]
	async fn middleware() {
		let mut world = World::new();
		world.spawn((
			RouteInfo::get("/"),
			RouteLayer::after_route(|world: &mut World| {
				let output = world
					.remove_resource::<RouteHandlerOutput<BoxedBundle>>()
					.unwrap();
				let entity = output.0.add_to_world(world);
				world.spawn((HtmlDocument, rsx! {
					middleware! {entity}
				}));
			}),
			RouteHandler::new_bundle(|| {
				rsx! {
					<MyTemplate foo=42/>
				}
			}),
		));

		BeetRouter::oneshot_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str(
				"<!DOCTYPE html><html><head></head><body>middleware!<div>foo: 42</div></body></html>",
			);
	}
}
