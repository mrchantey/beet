use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::as_beet::*;
use bevy::prelude::*;

/// A [`RouteHandler`] layer for converting bundles into HTML responses.
/// - First checks for a [`HtmlDocument`] and renders that one,
/// - otherwise searches for a [`HandlerBundle`].
pub fn bundle_layer() -> RouteHandler {
	RouteHandler::new(|world: &mut World| -> HttpResult<Response> {
		let entity = if let Some(&entity) = world
			.query_filtered_once::<Entity, With<HtmlDocument>>()
			.iter()
			.next()
		{
			entity
		} else {
			*world
				.query_filtered_once::<Entity, With<HandlerBundle>>()
				.iter()
				.next()
				.ok_or_else(|| HttpError::not_found())?
		};
		world.entity_mut(entity).insert(HtmlDocument);
		world.run_schedule(Update);
		let html = world.run_system_cached_with(render_fragment, entity)?;
		Ok(Html(html).into_response())
	})
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[template]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		rsx! {
			<div>foo: {foo}</div>
		}
	}

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin);
		let world = app.world_mut();
		world.spawn(children![
			RouteHandler::new_bundle(|| {
				rsx! {
					<MyTemplate foo=42/>
				}
			}),
			bundle_layer(),
		]);

		Router::oneshot_str(world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str(
				"<!DOCTYPE html><html><head></head><body><div>foo: 42</div></body></html>",
			);
	}
	#[sweet::test]
	async fn middleware() {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin);
		let world = app.world_mut();
		world.spawn(children![
			RouteHandler::new_bundle(|| {
				rsx! {
					<MyTemplate foo=42/>
				}
			}),
			RouteHandler::new_layer(|world: &mut World| {
				let entity = world
					.query_filtered_once::<Entity, With<HandlerBundle>>()[0];
				world.spawn((HtmlDocument, rsx! {
					"middleware!" {entity}
				}));
			}),
			bundle_layer(),
		]);

		Router::oneshot_str(world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str(
				"<!DOCTYPE html><html><head></head><body>middleware!<div>foo: 42</div></body></html>",
			);
	}
}
