use beet_core::prelude::*;
use beet_dom::prelude::BeetRoot;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// A [`SystemParam`] for retrieving the [`HtmlBundle`] for
/// a given `exchange`
#[derive(SystemParam)]
pub struct HtmlBundleQuery<'w, 's, F = ()>
where
	F: 'static + QueryFilter,
{
	exchanges: Query<'w, 's, &'static Children, F>,
	html_bundles: Query<'w, 's, Entity, With<HtmlBundle>>,
}

impl<F> HtmlBundleQuery<'_, '_, F>
where
	F: 'static + QueryFilter,
{
	/// Get the first [`HtmlBundle`] found in the direct children of the given `exchange`.
	/// Returns `None` if no [`HtmlBundle`] is found.
	/// ## Errors
	/// if multiple children are found.
	pub fn get(&self, exchange: Entity) -> Result<Option<Entity>> {
		let Ok(children) = self.exchanges.get(exchange) else {
			return Ok(None);
		};
		let found = children
			.iter()
			.filter_map(|e| self.html_bundles.get(e).ok())
			.collect::<Vec<_>>();

		match found.len() {
			0 => Ok(None),
			1 => Ok(Some(found[0])),
			_ => bevybail!(
				"Multiple HtmlBundle found in exchange children,
				this is usually caused by multiple matching endpoints.
				Please check each has a distinct Method and PathFilter"
			),
		}
	}
}


/// An action for converting a [`HtmlBundle`] into a HTML response,
/// only checking direct [`Children`] of the `exchange`.
/// If a response already exists or none are found this action does nothing.
///
/// ## Errors
///
/// Errors if multiple [`HtmlBundle`] are found.
pub fn html_bundle_to_response() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>,
		 mut commands: Commands,
		 query: HtmlBundleQuery<Without<Response>>|
		 -> Result {
			let exchange = ev.agent();
			let Some(html_bundle) = query.get(ev.agent())? else {
				ev.trigger_with_cx(Outcome::Fail);
				return Ok(());
			};

			commands.queue(move |world: &mut World| -> Result {
				// unless a [`BeetRoot`] is explicitly inserted,
				// we assume this fragment should be wrapped in
				// a [`HtmlDocument`], which also inserts a [`BeetRoot`]
				if !world.entity_mut(html_bundle).contains::<BeetRoot>() {
					world.entity_mut(html_bundle).insert(HtmlDocument);
				}
				world.run_schedule(ApplyDirectives);
				let html = world
					.run_system_cached_with(render_fragment, html_bundle)?;
				world
					.entity_mut(exchange)
					.insert(Html(html).into_response());
				Ok(())
			});
			ev.trigger_with_cx(Outcome::Pass);
			Ok(())
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
		RouterPlugin::world()
			.spawn((Router, Sequence, children![
				EndpointBuilder::get()
					.with_handler(|| (BeetRoot, rsx! {<div>hello world</div>})),
				html_bundle_to_response(),
			]))
			.oneshot_str(Request::get("/"))
			.await
			.xpect_eq("<div>hello world</div>");
	}


	#[sweet::test]
	async fn with_template() {
		RouterPlugin::world()
			// .with_resource(RenderMode::Ssr)
			.spawn((Router, Sequence, children![
				EndpointBuilder::get()
					.with_handler(|| rsx! {<MyTemplate foo=42/>}),
				html_bundle_to_response(),
			]))
			.oneshot_str(Request::get("/"))
			.await
			.xpect_eq(
				"<!DOCTYPE html><html><head></head><body><div>foo: 42</div></body></html>",
			);
	}
	#[sweet::test]
	async fn middleware() {
		RouterPlugin::world()
			// .with_resource(RenderMode::Ssr)
			.spawn((Router, Sequence, children![
				EndpointBuilder::get()
					.with_handler(|| rsx! {<MyTemplate foo=42/>}),
				OnSpawn::observe(
					|mut ev: On<GetOutcome>,
					 query: HtmlBundleQuery<Without<Response>>,
					 mut commands: Commands|
					 -> Result {
						let Some(html_bundle) = query.get(ev.agent())? else {
							ev.trigger_with_cx(Outcome::Fail);
							return Ok(());
						};
						commands.spawn((
							HtmlDocument,
							HtmlBundle,
							ChildOf(ev.agent()),
							rsx! {
								"middleware!" {html_bundle}
							},
						));
						ev.trigger_with_cx(Outcome::Pass);
						Ok(())
					}
				),
				html_bundle_to_response(),
			]))
			.oneshot_str(Request::get("/"))
			.await
			.xpect_str("<!DOCTYPE html><html><head></head><body>middleware!<div>foo: 42</div></body></html>");
	}
}
