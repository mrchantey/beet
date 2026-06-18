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
	pub template: SmolStr,
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
	pub fn new(template: impl Into<SmolStr>) -> Self {
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
	// configuration sits on the ancestor that declared it (eg the router). The
	// caller is also the in-tree anchor threaded into the render context.
	let route = cx.id();
	let template = cx
		.caller
		.with_state::<AncestorQuery<&BsxLayout>, Result<SmolStr>>(
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
			wrap_content_with(
				world,
				parts,
				route,
				content,
				|world, rendered| build_bsx_layout(world, &template, rendered),
			)
		})
		.await
}

/// Spawn the named BSX template as the layout around `rendered`: the slot child
/// carrying the content [`Portal`](beet_ui::prelude::Portal) spawns
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
		.spawn(children![(Portal::new(rendered), SlotChild::new())])
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
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"page body"</p> }
				)
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		html.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<main>")
			.xpect_contains("<p>page body</p>");
	}

	/// The shipped `<SiteLayout>` shell, used as a no-code site's layout template,
	/// lands the transcluded route body inside its `<main>` (below the header
	/// chrome) with no relay `<Slot/>`: the middleware routes the body as
	/// SiteLayout's default-slot child, which forwards into its own default slot.
	/// Regression for a bare-`<Slot/>` relay that leaked the body above the header.
	#[beet_core::test]
	async fn site_layout_routes_body_into_main() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// SiteLayout's Header/RouteHead read the site name off PackageConfig.
		world.init_resource::<PackageConfig>();
		let mut registry = BsxTemplateRegistry::default();
		registry.insert_source("Layout", "<SiteLayout/>").unwrap();
		world.insert_resource(registry);
		let root = world
			.spawn((Router, BsxLayout::default(), children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"page body"</p> }
				)
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		// the body sits inside <main>, not leaked above the header.
		let main_open = html.find("<main").unwrap();
		let main_close = html.find("</main>").unwrap();
		let body_at = html.find("page body").unwrap();
		(body_at > main_open && body_at < main_close).xpect_true();
	}

	/// The shipped `Layout.bsx` idiom verbatim: a leading comment then
	/// `<SiteLayout>` carrying slotted children that override named slots. The
	/// leading comment makes the layout document *multi-root*, so `<SiteLayout>`
	/// builds one level below the layout root (a tag-less wrapper). The transcluded
	/// body must still reach SiteLayout's default `<main>` slot (the wrapper
	/// forwards its default content into its lone template-invocation child) *and*
	/// the slotted children fill their named slots, all in one resolution pass.
	/// Regression for the body leaking into the head widget's default `<Slot/>` (or
	/// going unconsumed) when a nested widget exposed a competing default slot.
	#[beet_core::test]
	async fn site_layout_slotted_idiom_routes_body_and_slots() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.init_resource::<PackageConfig>();
		let mut registry = BsxTemplateRegistry::default();
		registry
			.insert_source(
				"Layout",
				"<!-- layout -->\n<SiteLayout>\n\t<meta slot=\"head\" name=\"x-custom\"/>\n\t<div slot=\"sidebar\">\"custom rail\"</div>\n</SiteLayout>",
			)
			.unwrap();
		world.insert_resource(registry);
		let root = world
			.spawn((Router, BsxLayout::default(), children![
				render_action::fixed_func_route("", || {
					let meta = ArticleMeta {
						title: Some("Welcome".into()),
						..default()
					};
					(meta, rsx! { <p>"page body"</p> })
				})
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		// the named-slot overrides filled their slots.
		html.as_str()
			.xpect_contains("custom rail")
			.xpect_contains("x-custom");
		// the body still routes into <main> alongside the overrides, not leaked into
		// the head slot beside the `x-custom` override.
		let main_open = html.find("<main").unwrap();
		let main_close = html.find("</main>").unwrap();
		let body_at = html.find("page body").unwrap();
		(body_at > main_open && body_at < main_close).xpect_true();
	}

	#[beet_core::test]
	async fn missing_template_errors() {
		let mut world = router_world();
		let root = world
			.spawn((Router, BsxLayout::new("Nope"), children![
				render_action::fixed_func_route("", || rsx! { <p>"body"</p> })
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
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"page body"</p> }
				)
			]))
			.flush();

		let first = get(&mut world, root, "").await;
		let second = get(&mut world, root, "").await;
		first.xpect_eq(second);
	}

	/// A world whose `Layout` binds its `<title>` from the transcluded route's
	/// `ArticleMeta` via the reserved `@entity:PageRoot::` selector. This is the
	/// in-markup replacement for the Rust `RouteHead` title lookup: the layout
	/// builds detached and the binding follows the `LayoutContent` link
	/// (installed by `wrap_content_with`) across the transclusion boundary.
	fn meta_layout_world() -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let mut registry = BsxTemplateRegistry::default();
		registry
			.insert_source(
				"Layout",
				"<html><head><title>{@entity:PageRoot::ArticleMeta.title}</title></head><body><main><Slot/></main></body></html>",
			)
			.unwrap();
		world.insert_resource(registry);
		world
	}

	/// A route whose rendered content carries the given frontmatter title.
	fn meta_route(path: &str, title: &str) -> impl Bundle {
		let meta = ArticleMeta {
			title: Some(title.into()),
			..default()
		};
		render_action::fixed_func_route(path, move || {
			(meta.clone(), rsx! { <p>"body"</p> })
		})
	}

	/// A layout-head `@entity:PageRoot::ArticleMeta.title` binding resolves to the
	/// transcluded route's meta, and differs per route (the gap this stream
	/// closes: the layout root's self-referential render root is not the content,
	/// so the walk must follow the distinct content link).
	#[beet_core::test]
	async fn layout_title_binds_transcluded_route_meta() {
		let mut world = meta_layout_world();
		let root = world
			.spawn((Router, BsxLayout::default(), children![
				meta_route("alpha", "Alpha"),
				meta_route("beta", "Beta"),
			]))
			.flush();

		// each route's meta surfaces in the layout head, transcluded by reference.
		get(&mut world, root, "alpha")
			.await
			.xpect_contains("<title>Alpha</title>");
		// a different route renders a different title through the same layout.
		get(&mut world, root, "beta")
			.await
			.xpect_contains("<title>Beta</title>");
	}

	/// The layout link [`LayoutContent`] (and its reverse [`LayoutContentOf`])
	/// survives a scene serialization round-trip: a reflect-registered
	/// relationship whose `#[entities]` source remaps onto the freshly spawned
	/// content, and whose collection target the relationship hook rebuilds on
	/// load. This is the link a reloaded layout tree's reserved bindings follow.
	#[cfg(all(feature = "template_serde", feature = "json"))]
	#[beet_core::test]
	fn layout_content_round_trips_through_scene() {
		// a tree the saver collects by `Children`: a content sibling and a layout
		// root linked to it via `LayoutContent`, so the edge and its reverse
		// collection both serialize and rebuild on load with remapped ids.
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world.spawn_empty().id();
		let content = world.spawn(ChildOf(root)).id();
		let layout = world
			.spawn((ChildOf(root), LayoutContent::new(content)))
			.id();
		world.flush();
		// the relationship hook mirrors the edge onto the content's reverse side.
		world
			.entity(content)
			.get::<LayoutContentOf>()
			.unwrap()
			.holders()
			.xpect_eq(&[layout]);

		let bytes = TemplateSaver::new()
			.with_entity_tree(&world, root)
			.save(&world, MediaType::Json)
			.unwrap();

		// load into a fresh world: the saved entity ids are remapped to new ones.
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		TemplateLoader::new(&mut world).load(&bytes).unwrap();
		// the reloaded layout link points at the reloaded content, and the content
		// carries the rebuilt reverse edge back to the layout.
		let (layout, layout_content) = world
			.query_once::<(Entity, &LayoutContent)>()
			.into_iter()
			.next()
			.unwrap();
		let content = layout_content.0;
		layout.xpect_not_eq(content);
		world
			.entity(content)
			.get::<LayoutContentOf>()
			.unwrap()
			.holders()
			.xpect_eq(&[layout]);
	}
}
