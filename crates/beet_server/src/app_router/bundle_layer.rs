use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::as_beet::*;
use bevy::prelude::*;


/// A system for converting bundles into HTML responses, automatically
/// run by the router if no [`Response`] is set.
/// - First checks for a [`HtmlDocument`] and renders that one,
/// - otherwise searches for a [`HandlerBundle`].
pub fn bundle_to_html(world: &mut World) -> HttpResult<Html> {
	let entity = if let Some(&entity) = world
		.query_filtered_once::<Entity, With<HtmlDocument>>()
		.iter()
		.next()
	{
		entity
	} else {
		let entity = *world
			.query_filtered_once::<Entity, With<HandlerBundle>>()
			.iter()
			.next()
			.ok_or_else(|| HttpError::not_found())?;
		world.entity_mut(entity).insert(HtmlDocument);
		entity
	};
	world.run_schedule(ApplySnippets);
	world.run_schedule(ApplyDirectives);
	let html = world.run_system_cached_with(render_fragment, entity)?;
	Ok(Html(html))
}

/// A [`RouteHandler`]

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
		Router::new(|app: &mut App| {
			app.world_mut().spawn(children![RouteHandler::bundle(
				HttpMethod::Get,
				|| {
					rsx! {
						<MyTemplate foo=42/>
					}
				}
			),]);
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str(
			"<!DOCTYPE html><html><head></head><body><div>foo: 42</div></body></html>",
		);
	}
	#[sweet::test]
	async fn middleware() {
		Router::new(|app: &mut App| {
			app.world_mut().spawn(children![
				RouteHandler::bundle(HttpMethod::Get, || {
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
			]);
		}).oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str(
				"<!DOCTYPE html><html><head></head><body>middleware!<div>foo: 42</div></body></html>",
			);
	}
}
