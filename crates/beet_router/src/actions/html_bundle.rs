use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// An action for converting a [`HandlerBundle`] into a HTML response,
/// only checking direct [`Children`] of the `exchange`.
/// If a response already exists or none are found this action does nothing.
///
/// ## Errors
///
/// Errors if multiple [`HandlerBundle`] are found.
pub fn html_bundle_to_response() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>,
		 mut commands: Commands,
		 exhanges: Query<&Children, Without<Response>>,
		 html_bundles: Query<Entity, With<HandlerBundle>>|
		 -> Result {
			let exchange = ev.agent();
			let Ok(children) = exhanges.get(exchange) else {
				ev.trigger_next(Outcome::Fail);
				return Ok(());
			};
			let found = children
				.iter()
				.filter_map(|e| html_bundles.get(e).ok())
				.collect::<Vec<_>>();

			match found.len() {
				0 => {
					ev.trigger_next(Outcome::Fail);
					Ok(())
				}
				1 => {
					let entity = found[0];
					commands.queue(move |world: &mut World| -> Result {
						world.run_schedule(ApplyDirectives);
						let html = world
							.run_system_cached_with(render_fragment, entity)?;
						world
							.entity_mut(exchange)
							.insert(Html(html).into_response());
						Ok(())
					});
					ev.trigger_next(Outcome::Pass);
					Ok(())
				}
				_ => bevybail!(
					"Multiple HandlerBundle found in exchange children,
					this is usually caused by multiple matching endpoints.
					Please check each has a distinct Method and PathFilter"
				),
			}
		},
	)
}

/// A [`RouteHandler`]

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[template]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		rsx! {
			<div>foo: {foo}</div>
		}
	}

	#[sweet::test]
	async fn simple() {
		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![
				EndpointBuilder::get()
					.with_handler(|| Html(rsx! {<div>hello world</div>})),
				html_bundle_to_response(),
			]))
			.oneshot_str(Request::get("/"))
			.await
			.xpect_eq("<div>hello world</div>");
	}


	#[sweet::test]
	async fn with_template() {
		todo!();
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
		todo!();
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
