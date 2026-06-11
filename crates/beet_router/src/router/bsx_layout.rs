//! The markup-declared counterpart of [`BaseLayout`](crate::prelude::BaseLayout):
//! the document layout is a BSX template resolved by name from the
//! [`BsxTemplateRegistry`], so a no-code site can choose its layout entirely
//! from markup, eg `<Router {BsxLayout{template:"Layout"}}>`.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Render middleware wrapping every descendant render route in the named BSX
/// template, with the route's content transcluded into the template's default
/// `<Slot/>` by reference (the [`BaseLayout`](crate::prelude::BaseLayout)
/// transclusion semantics).
///
/// The name resolves against the [`BsxTemplateRegistry`] at render time, eg
/// `Layout` for a `Layout.bsx` registered via `register_bsx_templates`. Add it
/// to an ancestor of the routes it should wrap, typically the router entity.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(BsxLayoutAction)]
pub struct BsxLayout {
	/// The BSX-registry template name, eg `Layout` or `path::to::Layout`.
	pub template: String,
}

impl Default for BsxLayout {
	fn default() -> Self {
		Self {
			template: "Layout".into(),
		}
	}
}

impl BsxLayout {
	/// Wrap descendant routes in the named BSX template.
	pub fn new(template: impl Into<String>) -> Self {
		Self {
			template: template.into(),
		}
	}
}

/// The render middleware behind [`BsxLayout`]: runs the inner handler, then
/// builds the named BSX template with the content routed into its default slot.
#[action]
#[derive(Default, Component)]
#[component(on_add = on_add_middleware::<Self, RequestParts, Entity>)]
async fn BsxLayoutAction(
	cx: ActionContext<(RequestParts, Next<RequestParts, Entity>)>,
) -> Result<Entity> {
	let (parts, next) = &cx.input;
	// resolve the inner content render root, then wrap it
	let content = next.call(parts.clone()).await?;
	let parts = parts.clone();
	// the middleware runs with the matched route as caller; the `BsxLayout`
	// configuration sits on the ancestor that declared it (eg the router)
	let template = cx
		.caller
		.with_state::<AncestorQuery<&BsxLayout>, Result<String>>(
			|entity, query| {
				query
					.get(entity)
					.map(|layout| layout.template.clone())
					.map_err(|_| bevyhow!("no ancestor `BsxLayout` found"))
			},
		)
		.await??;
	next.world()
		.clone()
		.with(move |world: &mut World| {
			wrap_content_with(world, parts, content, |world, rendered| {
				build_bsx_layout(world, &template, rendered)
			})
		})
		.await
}

/// Spawn the named BSX template as the layout around `rendered`: the slot child
/// carrying the content [`RenderRef`](beet_ui::prelude::RenderRef) spawns
/// first, then the template builds into the same entity, whose slot-resolution
/// pass routes the content into the template's default `<Slot/>`.
fn build_bsx_layout(
	world: &mut World,
	template: &str,
	rendered: Entity,
) -> Result<Entity> {
	use beet_ui::prelude::*;
	let registry = world
		.get_resource::<BsxTemplateRegistry>()
		.cloned()
		.unwrap_or_default();
	let nodes = registry
		.get(template)
		.ok_or_else(|| {
			bevyhow!("no BSX template registered under `{template}`")
		})?
		.nodes
		.clone();
	let layout = world
		.spawn(children![(RenderRef::new(rendered), SlotChild::new())])
		.id();
	world
		.entity_mut(layout)
		.insert_template(BsxTemplate::new(nodes, registry))?;
	layout.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let mut registry = BsxTemplateRegistry::default();
		registry
			.insert_source(
				"Layout",
				"<html><head><meta charset=\"utf-8\"/></head><body><main><Slot/></main></body></html>",
			)
			.unwrap();
		world.insert_resource(registry);
		world
	}

	/// Request `path`, negotiating HTML, and return the rendered body.
	async fn get(world: &mut World, root: Entity, path: &str) -> String {
		world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::get(path)
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
	}

	#[beet_core::test]
	async fn wraps_route_in_named_template() {
		let mut world = router_world();
		let root = world
			.spawn((Router, BsxLayout::default(), children![
				render_action::fixed_route("", rsx! { <p>"page body"</p> })
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		html.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<main>")
			.xpect_contains("<p>page body</p>");
	}

	#[beet_core::test]
	async fn missing_template_errors() {
		let mut world = router_world();
		let root = world
			.spawn((Router, BsxLayout::new("Nope"), children![
				render_action::fixed_route("", rsx! { <p>"body"</p> })
			]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[beet_core::test]
	async fn repeat_requests_render_identically() {
		let mut world = router_world();
		let root = world
			.spawn((Router, BsxLayout::default(), children![
				render_action::fixed_route("", rsx! { <p>"page body"</p> })
			]))
			.flush();

		let first = get(&mut world, root, "").await;
		let second = get(&mut world, root, "").await;
		first.xpect_eq(second);
	}
}
