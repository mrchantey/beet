use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// Marker type indicating this entity was spawned via [`bundle_endpoint`].
#[derive(Component)]
pub struct HandlerBundle;


/// A route handler returning a bundle, which is inserted into the world
/// with a [`HandlerBundle`] component.
pub fn bundle_endpoint<T, In, InErr, Out, Marker>(handler: T) -> impl Bundle
where
	T: 'static + Send + Sync + Clone + IntoSystem<In, Out, Marker>,
	In: 'static + SystemInput,
	for<'a> In::Inner<'a>: TryFrom<Request, Error = InErr>,
	InErr: IntoResponse,
	Out: 'static + Send + Sync + Bundle,
{
	let handler = move |world: &mut World| -> Result<(), Response> {
		let input = world
			.remove_resource::<Request>()
			.ok_or_else(|| no_request_err::<T>())?
			.try_into()
			.map_err(|err: InErr| err.into_response())?;
		match world.run_system_cached_with(handler.clone(), input) {
			Ok(out) => {
				world.spawn((HandlerBundle, out));
				// world.run_schedule(ApplySnippets);
			}
			Err(err) => {
				world.insert_resource(HttpError::from(err).into_response());
			}
		}
		Ok(())
	};

	(
		Endpoint,
		RouteHandler::layer(move |world: &mut World| {
			if let Err(err) = handler(world) {
				world.insert_resource(err);
			}
		}),
	)
}


/// An async route handler returning a bundle, which is inserted into the world
/// with a [`HandlerBundle`] component.
pub fn bundle_endpoint_async<Handler, Fut, Out>(handler: Handler) -> impl Bundle
where
	Handler: 'static + Send + Sync + Clone + FnOnce(World) -> Fut,
	Fut: 'static + Send + Future<Output = (World, Out)>,
	Out: 'static + Send + Sync + Bundle,
{
	(
		Endpoint,
		RouteHandler::layer_async(move |world, _| {
			let func = handler.clone();
			async move {
				let (mut world, out) = func(world).await;
				world.spawn((HandlerBundle, out));
				world
			}
		}),
	)
}


/// A system for converting bundles into HTML responses, automatically
/// run by the router if no [`Response`] is set.
/// - First checks for a [`HtmlDocument`] and renders that one,
/// - otherwise searches for a [`HandlerBundle`].
pub fn bundle_to_html_handler() -> impl Bundle {
	RouteHandler::layer(system.pipe(insert_response_if_error))
}

fn insert_response_if_error(In(result): In<Result>, mut commands: Commands) {
	if let Err(err) = result {
		commands.insert_resource(err.into_response());
	}
}


fn system(world: &mut World) -> Result {
	let entity = if let Some(&entity) = world
		.query_filtered_once::<Entity, With<HtmlDocument>>()
		.iter()
		.next()
	{
		entity
	} else if let Some(&entity) = world
		.query_filtered_once::<Entity, With<HandlerBundle>>()
		.iter()
		.next()
	{
		world.entity_mut(entity).insert(HtmlDocument);
		entity
	} else {
		return Ok(());
	};
	world.run_schedule(ApplySnippets);
	world.run_schedule(ApplyDirectives);
	let html = world.run_system_cached_with(render_fragment, entity)?;
	world.insert_resource(Html(html).into_response());
	Ok(())
}

/// A [`RouteHandler`]

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	fn some_endpoint(_: Query<()>) -> impl Bundle + use<> {
		rsx! {
			<div>some endpoint</div>
		}
	}

	#[template]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		rsx! {
			<div>foo: {foo}</div>
		}
	}
	#[sweet::test]
	fn compiles() { bundle_endpoint(some_endpoint); }

	#[sweet::test]
	async fn works() {
		Router::new_bundle(|| {
			bundle_endpoint(|| {
				rsx! {
					<MyTemplate foo=42/>
				}
			})
		})
		.with_resource(RenderMode::Ssr)
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect_str(
			"<!DOCTYPE html><html><head></head><body><div>foo: 42</div></body></html>",
		);
	}
	#[sweet::test]
	async fn middleware() {
		Router::new_bundle(|| {
			children![
				bundle_endpoint(|| {
					rsx! {
						<MyTemplate foo=42/>
					}
				}),
				RouteHandler::layer(|world: &mut World| {
					let entity = world
						.query_filtered_once::<Entity, With<HandlerBundle>>()[0];
					world.spawn((HtmlDocument, rsx! {
						"middleware!" {entity}
					}));
				}),
			]
		})
		.with_resource(RenderMode::Ssr)
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect_str("<!DOCTYPE html><html><head></head><body>middleware!<div>foo: 42</div></body></html>");
	}
}
