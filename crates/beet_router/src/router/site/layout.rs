//! Layout render middleware: wrap a route's rendered content in a document
//! layout (the web `<html>`/`<head>` document, an article shell, a terminal
//! column) without reparenting or re-resolving it.
//!
//! [`Layout`] is a render-middleware component (registered like any other
//! middleware, eg [`RequestLogger`]). For every descendant render route it runs
//! the inner handler to obtain the content render root, then builds the named
//! layout with that content routed into its default `<slot>` as a [`Portal`]
//! transclusion. The content is rendered *in place, by reference*: it is never
//! reparented under the layout nor re-resolved, so a persistent fixed route
//! survives request after request.
//!
//! The layout wraps **every** request regardless of target. Non-visual document
//! chrome (`<head>`/`<style>`/`<script>`) simply does not paint in the terminal
//! (it resolves to `display: none`; see the user-agent style layer), so the same
//! layout renders correctly on web and terminal.
//!
//! ## Nesting
//!
//! Layouts nest: every ancestor declaring one wraps the route exactly once, and
//! the **furthest ancestor is outermost**. So a site `Layout` on the router and
//! an `ArticleLayout` on the blog route render as site(article(body)), and the
//! article layout is inner chrome only rather than a second document shell.
//!
//! Ordering is [`MiddlewareQuery::resolve_action`](crate::prelude::MiddlewareList)'s
//! doing, wrapping leaf-first so the root ancestor lands outermost; each
//! instance reads the [`Layout`] of the entity that DECLARED it, never the
//! nearest one to the route.
//!
//! ## Main content
//!
//! A site's layout marks its main content with a `<main>`, as the web does,
//! and the `--root` render param reads it ([`RenderRoot`], whose module docs
//! hold the cascade). Everything outside the `<main>` is chrome: the head, the
//! header and nav, the sidebar, the footer. So `--root=main` answers the
//! `<main>` and what a layout put inside it, `--root=content` only the route
//! content the layouts transclude, and no `--root` the whole page.
//!
//! The two differ by what an inner layout adds inside `<main>`. The site's
//! `ArticleLayout` renders `ArticleHeader` (the `<h1>` title, the byline and
//! the companion video, all from `DocMeta`) above the post: a `main` render
//! keeps that header, which a reader asking for the main content wants, and a
//! `content` render leaves it out, which every syndication target wants since
//! its record already carries the title and dates.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Render middleware wrapping every descendant render route in a layout, with
/// the route's content transcluded into its default `<slot/>` by reference.
///
/// Add it to an ancestor of the routes it should wrap, typically the router
/// entity, exactly like any other middleware ([`RequestLogger`], [`HelpHandler`]):
///
/// ```bsx
/// <Router {Layout(Document("templates/Layout.bsx"))}>
///     <RoutesDir src="routes"/>
///     <Route path="blog" {Layout(Document("templates/ArticleLayout.bsx"))}>..</Route>
/// </Router>
/// ```
///
/// What it names is a [`TemplateSource`]: a document of the package the
/// declaration is in, activated around each page, or a rust `#[template]` by
/// its type path. One that names nothing fails the request rather than serving
/// an unwrapped page.
///
/// From rust a template's name comes from the type ([`Layout::of`]), so it is a
/// symbol the compiler checks:
///
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_ui::prelude::*;
/// #[template]
/// fn PageShell() -> impl Bundle { rsx! { <html><body><slot/></body></html> } }
/// let bundle = (Router, Layout::of::<PageShell>());
/// ```
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = push_layout_middleware)]
pub struct Layout(pub TemplateSource);

impl Layout {
	/// Wrap descendant routes in the rust template `T` ([`TemplateSource::of`]).
	pub fn of<T: BuildTemplate>() -> Self { Self(TemplateSource::of::<T>()) }

	/// Wrap descendant routes in the rust template named `template`. Prefer
	/// [`of`](Self::of), which derives the name from the type.
	pub fn template(template: impl Into<SmolStr>) -> Self {
		Self(TemplateSource::template(template))
	}

	/// Wrap descendant routes in the document at `document` of their package.
	pub fn document(document: impl Into<RelPath>) -> Self {
		Self(TemplateSource::document(document))
	}
}

/// Component hook: push this declaration onto its own entity's
/// [`MiddlewareList`], so every route dispatched at or beneath it renders
/// through the layout.
///
/// The pushed closure captures the DECLARING entity rather than resolving a
/// [`Layout`] by ancestry at render time: two nested declarations produce two
/// wrappers, and each must build the layout *it* names.
fn push_layout_middleware(mut world: DeferredWorld, cx: HookContext) {
	let declarer = cx.entity;
	world.commands().entity(declarer).queue(
		move |mut entity: EntityWorldMut| {
			entity
				.get_mut_or_default::<MiddlewareList<RequestParts, Entity>>()
				.add(
					move |parts: RequestParts,
					      next: Next<RequestParts, Entity>| {
						async move {
							// resolve the inner content render root, then wrap it
							let content = next.call(parts.clone()).await?;
							// the middleware runs with the matched route as caller, the
							// reliable in-tree anchor for tree-scoped widgets (the
							// rendered content may be detached)
							let route = next.id();
							// read per request, so a live edit to the declaration
							// or its document is the next request's
							let declarer = next.world().entity(declarer);
							let layout = declarer
								.get::<Layout, _>(Layout::clone)
								.await?;
							let source = layout.0.load(&declarer).await?;
							next.world()
								.clone()
								.with(move |world: &mut World| {
									wrap_content(
										world, parts, route, content, source,
									)
								})
								.await
						}
					},
				);
		},
	);
}

/// Wrap the inner render root `content` in `source`: push the request-scoped
/// [`RequestContext`] onto the [`RequestContextStack`] around the layout build
/// (which spawns the layout with the content transcluded into its default slot),
/// then mark the layout as the new render root.
///
/// The layout subtree is ephemeral and despawned after render (along with the
/// content's own ephemerals), but the referenced content is never owned or
/// despawned here, so a persistent fixed route survives request after request.
fn wrap_content(
	world: &mut World,
	parts: RequestParts,
	route: Entity,
	content: Entity,
	source: LoadedTemplate,
) -> Result<Entity> {
	// the inner render root names the entity to render and its ephemerals
	let (rendered, content_despawn) = {
		let entity = world.entity(content);
		let rendered = entity
			.get::<PageRoot>()
			.ok_or_else(|| {
				bevyhow!("layout inner handler did not yield a render root")
			})?
			.rendered();
		let despawn = entity
			.get::<DespawnAfterRender>()
			.map(|despawn| despawn.0.clone())
			.unwrap_or_default();
		(rendered, despawn)
	};
	// what this layout STRUCTURALLY wraps may itself be a layout (a nested
	// declaration), while every consumer of the link wants the route content at
	// the end of that chain: the per-route components a layout widget queries
	// (`DocMeta`) live there, not on an intermediate shell.
	let page_content = LayoutContent::terminal(world, rendered)?;

	// resolve the entity owning this request's route tree once, as the nearest
	// tree-bearing ancestor of the in-tree route anchor (`route` is always in the
	// served tree, where `rendered` content may be detached). Threading this
	// handle lets tree-scoped widgets read the tree with an O(1) get instead of
	// re-walking each render; falling back to `route` when no tree ancestor exists
	// (eg a synthetic test root) leaves such a widget's lookup empty, not wrong.
	let router = world
		.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
			trees.get_entity(route)
		})
		.unwrap_or(route);

	// the request-scoped render context, read by the layout's scene systems: the
	// request parts, the rendered content entity (off which widgets query
	// per-route components, eg `DocMeta` parsed from frontmatter), the matched
	// route entity, and its tree-owning `router`. Scoped to the synchronous layout
	// build — a stack, so the content's own context (pushed around its build)
	// and any nested render are restored on completion.
	let layout = RequestContextStack::scoped(
		world,
		RequestContext::new(parts, page_content, route, router),
		|world| build_layout(world, source, rendered),
	)?;

	// link the layout root to the transcluded content, distinct from the
	// self-referential render root: a layout-head `@entity:PageRoot::` binding
	// follows this to read the route's `DocMeta` across the transclusion.
	world
		.entity_mut(layout)
		.insert(LayoutContent::new(page_content));

	// despawn the layout subtree plus the content's ephemerals after render
	let mut to_despawn = vec![layout];
	to_despawn.extend(content_despawn);
	PageRoot::insert(&mut world.entity_mut(layout), to_despawn);
	layout.xok()
}

/// Build `source` around `rendered`: the slot child carrying the content
/// [`Portal`] spawns first, then the layout builds below the same entity, whose
/// slot-resolution pass routes the content into the layout's default `<slot/>`.
///
/// Below it always, since a layout is a DOCUMENT: a passthrough whose whole body
/// is `<slot/>` would otherwise land its `SlotTarget` on the very entity holding
/// the transcluded content, where slot resolution cannot see it.
fn build_layout(
	world: &mut World,
	source: LoadedTemplate,
	rendered: Entity,
) -> Result<Entity> {
	let layout = world
		.spawn(children![(
			beet_ui::prelude::Portal::new(rendered),
			SlotChild::new()
		)])
		.id();
	source.build(&mut world.entity_mut(layout))?;
	layout.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;
	use beet_ui::prelude::DocMeta;
	// the `MetaLayout` test template's site-title link, gated like its only user.
	// Named rather than glob-imported: `beet_net` has a `Header` trait too.
	#[cfg(feature = "json")]
	use beet_ui::prelude::Header;

	/// A document layout with a `<meta charset>` head; the content fills `<main>`.
	#[template]
	fn PageShell() -> impl Bundle {
		rsx! {
			<html>
				<head><meta charset="utf-8"/></head>
				<body><main><slot/></main></body>
			</html>
		}
	}

	/// A layout that places the content inside `<nav>`.
	#[template]
	fn NavLayout() -> impl Bundle {
		rsx! { <body><nav><slot/></nav></body> }
	}

	/// A world whose rust test layouts resolve by name, the registration a
	/// `#[template]` layout needs to be nameable from a [`Layout`] declaration.
	fn router_world() -> World {
		let mut app = App::new().with_plugins((AsyncPlugin, RouterPlugin));
		app.install_types::<(PageShell, NavLayout)>();
		#[cfg(feature = "json")]
		app.install_types::<MetaLayout>();
		core::mem::take(app.world_mut())
	}

	/// A store of `documents`, each a layout by its path.
	async fn documents(documents: &[(&str, &str)]) -> BlobStore {
		let store = BlobStore::temp();
		for (path, source) in documents {
			store
				.insert(&RelPath::new(path), source.to_string())
				.await
				.unwrap();
		}
		store
	}

	/// A single route serving one page body under the `Layout.bsx` `source`.
	async fn document_root(world: &mut World, source: &str) -> Entity {
		let store = documents(&[("Layout.bsx", source)]).await;
		let root = layout_root(world, Layout::document("Layout.bsx"));
		world.entity_mut(root).insert(store);
		root
	}

	/// The plain document shell as a `.bsx` document, the no-code counterpart of
	/// [`PageShell`].
	const LAYOUT_BSX: &str = "<html><head><meta charset=\"utf-8\"/></head><body><main><slot/></main></body></html>";

	/// Request `path`, negotiating HTML, and return the rendered body.
	async fn get(world: &mut World, root: Entity, path: &str) -> String {
		world
			.entity_mut(root)
			.exchange(
				Request::get(path)
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap_str()
			.await
	}

	/// A single route serving one page body under `layout`.
	fn layout_root(world: &mut World, layout: Layout) -> Entity {
		world
			.spawn((Router, layout, children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"page body"</p> }
				)
			]))
			.flush()
	}

	/// A rust `#[template]` and a document wrap a page alike.
	#[beet_core::test]
	async fn wraps_route_in_a_rust_template() {
		let mut world = router_world();
		let root = layout_root(&mut world, Layout::of::<PageShell>());
		get(&mut world, root, "")
			.await
			.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<p>page body</p>");
	}

	#[beet_core::test]
	async fn wraps_route_in_a_document() {
		let mut world = router_world();
		let root = document_root(&mut world, LAYOUT_BSX).await;
		get(&mut world, root, "")
			.await
			.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<main>")
			.xpect_contains("<p>page body</p>");
	}

	#[beet_core::test]
	async fn layout_places_content_where_it_chooses() {
		// the layout decides placement; here the content lands inside <nav>
		let mut world = router_world();
		let root = world
			.spawn((Router, Layout::of::<NavLayout>(), children![
				render_action::fixed_func_route("", || rsx! { <a>"home"</a> })
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		let nav_open = html.find("<nav>").unwrap();
		let nav_close = html.find("</nav>").unwrap();
		let link = html.find("<a>home</a>").unwrap();
		link.xpect_greater_than(nav_open);
		link.xpect_less_than(nav_close);
	}

	#[beet_core::test]
	async fn fixed_route_survives_repeat_requests() {
		// the shared fixed content must not be despawned with the layout; each
		// request must render identically (the despawn-hazard regression).
		let mut world = router_world();
		let root = layout_root(&mut world, Layout::of::<PageShell>());
		let first = get(&mut world, root, "").await;
		let second = get(&mut world, root, "").await;
		second.as_str().xpect_contains("<p>page body</p>");
		first.xpect_eq(second);
	}

	#[beet_core::test]
	async fn wraps_async_route() {
		async fn page(_cx: ActionContext<Request>) -> impl Bundle {
			rsx! { <p>"async body"</p> }
		}
		let mut world = router_world();
		let root = world
			.spawn((Router, Layout::of::<PageShell>(), children![
				render_action::async_route("", page)
			]))
			.flush();

		// per-request content is ephemeral; render twice to prove cleanup
		for _ in 0..2 {
			get(&mut world, root, "")
				.await
				.as_str()
				.xpect_contains("<meta charset=\"utf-8\"")
				.xpect_contains("<p>async body</p>");
		}
	}

	#[beet_core::test]
	async fn wraps_blob_page_markdown() {
		let store = BlobStore::temp();
		store
			.insert(&"post.md".into(), "# Hello\n\nmarkdown body".to_owned())
			.await
			.unwrap();

		let mut world = router_world();
		let root = world
			.spawn((store, Router, Layout::of::<PageShell>(), children![
				route::new("post", BlobPage::new("post.md"))
			]))
			.flush();

		// the markdown content (parsed per request) lands inside the layout's
		// `main`, transcluded by reference
		get(&mut world, root, "post")
			.await
			.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("markdown body");
	}

	/// Repeated requests to the same markdown route must not leak entities: the
	/// per-request layout subtree and content ephemerals are despawned after
	/// render, and the persistent route tree is diffed in place. A growing entity
	/// count is the ramp-up bug (a page got slower with every visit because the
	/// post-parse pipeline re-scanned ever more resident entities).
	#[beet_core::test]
	async fn repeated_requests_stay_bounded() {
		let store = BlobStore::temp();
		store
			.insert(
				&"post.md".into(),
				"# Title\n\n```rust\nfn main() {}\n```\n\nbody".to_owned(),
			)
			.await
			.unwrap();

		let mut world = router_world();
		let root = world
			.spawn((store, Router, Layout::of::<PageShell>(), children![
				route::new("post", BlobPage::new("post.md"))
			]))
			.flush();

		// warm up so the route's tree is parsed and any one-off resources settle,
		// then sample the entity count and confirm it holds flat across requests.
		get(&mut world, root, "post").await;
		let baseline = world.iter_entities().count();
		for _ in 0..8 {
			get(&mut world, root, "post").await;
		}
		world.iter_entities().count().xpect_eq(baseline);
	}

	#[beet_core::test]
	async fn missing_layout_errors() {
		for layout in [Layout::template("Nope"), Layout::document("Nope.bsx")] {
			let mut world = router_world();
			let root = layout_root(&mut world, layout);
			world.entity_mut(root).insert(BlobStore::temp());
			world
				.entity_mut(root)
				.exchange(Request::get(""))
				.await
				.status()
				.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		}
	}

	/// The shipped `<SiteLayout>` shell, used as a no-code site's layout template,
	/// lands the transcluded route body inside its `<main>` (below the header
	/// chrome) with no relay `<slot/>`: the middleware routes the body as
	/// SiteLayout's default-slot child, which forwards into its own default slot.
	/// Regression for a bare-`<slot/>` relay that leaked the body above the header.
	#[beet_core::test]
	async fn site_layout_routes_body_into_main() {
		let mut world = router_world();
		// SiteLayout's Header/RouteHead read the site name off PackageConfig.
		world.init_resource::<PackageConfig>();
		let root = document_root(&mut world, "<SiteLayout/>").await;

		let html = get(&mut world, root, "").await;
		// the body sits inside <main>, not leaked above the header.
		assert_within_main(&html, "page body");
	}

	/// The body must land inside the layout's `<main>` rather than above the
	/// header or up in the non-visual `<head>`.
	fn assert_within_main(html: &str, body: &str) {
		let main_open = html.find("<main").unwrap();
		let main_close = html.find("</main>").unwrap();
		let body_at = html.find(body).unwrap();
		(body_at > main_open && body_at < main_close).xpect_true();
	}

	/// The shipped `Layout.bsx` idiom verbatim: a leading comment then
	/// `<SiteLayout>` carrying slotted children that override named slots. The
	/// leading comment makes the layout document *multi-root*, so `<SiteLayout>`
	/// builds one level below the layout root (a tag-less wrapper). The transcluded
	/// body must still reach SiteLayout's default `<main>` slot (the wrapper
	/// forwards its default content into its lone template-invocation child) *and*
	/// the slotted children fill their named slots, all in one resolution pass.
	/// Regression for the body leaking into the head widget's default `<slot/>` (or
	/// going unconsumed) when a nested widget exposed a competing default slot.
	const SLOTTED_LAYOUT_BSX: &str = "<!-- layout -->\n<SiteLayout>\n\t<meta slot=\"head\" name=\"x-custom\"/>\n\t<div slot=\"sidebar\">\"custom rail\"</div>\n</SiteLayout>";

	#[beet_core::test]
	async fn site_layout_slotted_idiom_routes_body_and_slots() {
		let mut world = router_world();
		world.init_resource::<PackageConfig>();
		let root = document_root(&mut world, SLOTTED_LAYOUT_BSX).await;

		let html = get(&mut world, root, "").await;
		// the named-slot overrides filled their slots.
		html.as_str()
			.xpect_contains("custom rail")
			.xpect_contains("x-custom");
		// the body still routes into <main> alongside the overrides, not leaked into
		// the head slot beside the `x-custom` override.
		assert_within_main(&html, "page body");
	}

	/// The doubly-nested multi-root case: the `Layout.bsx` idiom (a leading comment
	/// before `<SiteLayout>`) wrapping the shipped `<SiteLayout>`, which itself is
	/// multi-root (its `<!DOCTYPE html>` sits before `<html>`). So `<html>` builds
	/// two tag-less wrappers below the layout root, and the transcluded body must
	/// still anchor at `<html>` to reach `<main>`'s default slot rather than widen
	/// its scope into `RouteHead`'s default slot (first in document order). On the
	/// web that head-leak is merely invisible; on the terminal `<head>` is
	/// `display:none`, so the misrouted body vanishes entirely. Regression for the
	/// body landing in the head instead of `<main>` when nested layouts are
	/// multi-root.
	#[beet_core::test]
	async fn site_layout_nested_multiroot_routes_body_into_main() {
		let mut world = router_world();
		world.init_resource::<PackageConfig>();
		let root = document_root(&mut world, SLOTTED_LAYOUT_BSX).await;
		// the web lands the body inside <main>, not leaked up into the <head>.
		assert_within_main(&get(&mut world, root, "").await, "page body");
		// the terminal must keep the body too: a body misrouted into the non-visual
		// <head> would paint nothing, so `contains` is the meaningful terminal check.
		world
			.entity_mut(root)
			.exchange(
				Request::get("")
					.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap_str()
			.await
			.as_str()
			.xpect_contains("page body");
	}

	/// The nesting semantics: an ancestor's layout is OUTERMOST, each declaration
	/// applies exactly once, and the route body sits inside both. The
	/// double-wrap regression is the inner template applied twice (each middleware
	/// instance resolving the nearest `Layout` rather than its own declaration).
	#[beet_core::test]
	async fn nested_layouts_wrap_outer_then_inner() {
		let mut world = router_world();
		let store = documents(&[
			(
				"Outer.bsx",
				"<html><body><main data-outer><slot/></main></body></html>",
			),
			("Inner.bsx", "<article data-inner><slot/></article>"),
		])
		.await;
		let root = world
			.spawn((store, Router, Layout::document("Outer.bsx"), children![(
				PathPartial::new("nested"),
				Layout::document("Inner.bsx"),
				children![render_action::fixed_func_route(
					"page",
					|| rsx! { <p>"page body"</p> }
				)]
			)]))
			.flush();

		let html = get(&mut world, root, "nested/page").await;
		// each declaration applied exactly once
		html.matches("data-outer").count().xpect_eq(1);
		html.matches("data-inner").count().xpect_eq(1);
		// outer(inner(body)): the furthest ancestor is the outermost wrapper
		let outer = html.find("data-outer").unwrap();
		let inner = html.find("data-inner").unwrap();
		let body = html.find("page body").unwrap();
		inner.xpect_greater_than(outer);
		body.xpect_greater_than(inner);
	}

	/// The shapes a layout document may take: the `<slot/>` is not always inside
	/// an element, and the body must reach it in every one.
	///
	/// A chrome-only layout (`<ArticleHeader/><slot/>`) is the regression this
	/// covers: a multi-root document anchored its transcluded body inside the
	/// FIRST element it found, a sibling of the slot the body belonged in, and a
	/// bare `<slot/>` passthrough built its target onto the very entity holding
	/// the content, where slot resolution could not see it.
	#[beet_core::test]
	async fn layout_shapes_all_receive_the_body() {
		for source in [
			// the whole layout is the passthrough
			"<slot/>",
			// chrome beside the slot, at the document root
			"<p>chrome</p><slot/>",
			"<slot/><p>chrome</p>",
			// ..and under a tag-less host
			"<Fragment><p>chrome</p><slot/></Fragment>",
			// the ordinary element-wrapped shapes
			"<main><p>chrome</p><slot/></main>",
			"<html><body><main><slot/></main></body></html>",
			// a leading comment nests the content element below the root
			"<!-- c --><html><body><main><slot/></main></body></html>",
		] {
			let mut world = router_world();
			let root = document_root(&mut world, source).await;
			get(&mut world, root, "").await.xpect_contains("page body");
		}
	}

	/// A route under only the router's layout is untouched by a sibling subtree's
	/// declaration, and each sibling subtree gets its own.
	#[beet_core::test]
	async fn sibling_subtrees_get_their_own_layouts() {
		let mut world = router_world();
		let store = documents(&[
			("Outer.bsx", "<main data-outer><slot/></main>"),
			("Blog.bsx", "<article data-blog><slot/></article>"),
			("Docs.bsx", "<section data-docs><slot/></section>"),
		])
		.await;
		let page = || {
			render_action::fixed_func_route("page", || rsx! { <p>"body"</p> })
		};
		let root = world
			.spawn((store, Router, Layout::document("Outer.bsx"), children![
				(
					PathPartial::new("blog"),
					Layout::document("Blog.bsx"),
					children![page()]
				),
				(
					PathPartial::new("docs"),
					Layout::document("Docs.bsx"),
					children![page()]
				),
				page(),
			]))
			.flush();

		let blog = get(&mut world, root, "blog/page").await;
		blog.as_str()
			.xpect_contains("data-blog")
			.xnot()
			.xpect_contains("data-docs");
		let docs = get(&mut world, root, "docs/page").await;
		docs.as_str()
			.xpect_contains("data-docs")
			.xnot()
			.xpect_contains("data-blog");
		// the bare route wears the router's layout and nothing else
		let bare = get(&mut world, root, "page").await;
		bare.as_str()
			.xpect_contains("data-outer")
			.xnot()
			.xpect_contains("data-blog");
	}

	/// A store whose `Layout.bsx` binds its `<title>` from the transcluded
	/// route's `DocMeta` via the reserved `@entity:PageRoot::` selector. This is
	/// the in-markup replacement for the Rust `RouteHead` title lookup: the
	/// layout builds detached and the binding follows the `LayoutContent` link
	/// (installed by `wrap_content`) across the transclusion boundary.
	async fn meta_layout_store() -> BlobStore {
		documents(&[
			(
				"Layout.bsx",
				"<html><head><title>{@entity:PageRoot::DocMeta.title}</title></head><body><main><slot/></main></body></html>",
			),
			("ArticleLayout.bsx", "<article><slot/></article>"),
		])
		.await
	}

	/// A route whose rendered content carries the given frontmatter title.
	fn meta_route(path: &str, title: &str) -> impl Bundle {
		let meta = DocMeta {
			title: Some(title.into()),
			..default()
		};
		render_action::fixed_func_route(path, move || {
			(meta.clone(), rsx! { <p>"body"</p> })
		})
	}

	/// A layout-head `@entity:PageRoot::DocMeta.title` binding resolves to the
	/// transcluded route's meta, and differs per route (the gap this stream
	/// closes: the layout root's self-referential render root is not the content,
	/// so the walk must follow the distinct content link).
	#[beet_core::test]
	async fn layout_title_binds_transcluded_route_meta() {
		let mut world = router_world();
		let store = meta_layout_store().await;
		let root = world
			.spawn((store, Router, Layout::document("Layout.bsx"), children![
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

	/// The nested-layout title hop: the site layout's head binds the ROUTE's meta
	/// through the intervening article layout, rather than the article layout root
	/// (which carries no `DocMeta`, so the title would go blank).
	#[beet_core::test]
	async fn layout_title_binds_through_a_nested_layout() {
		let mut world = router_world();
		let store = meta_layout_store().await;
		let root = world
			.spawn((store, Router, Layout::document("Layout.bsx"), children![
				(
					PathPartial::new("blog"),
					Layout::document("ArticleLayout.bsx"),
					children![meta_route("post", "Post")]
				)
			]))
			.flush();

		get(&mut world, root, "blog/post")
			.await
			.xpect_contains("<title>Post</title>");
	}

	/// The real site head/header layout: `RouteHead` owns the single `<title>`
	/// bound to the route, `Header` owns the site-title link.
	#[cfg(feature = "json")]
	#[template]
	fn MetaLayout() -> impl Bundle {
		rsx! {
			<html>
				<RouteHead/>
				<body><Header/><main><slot/></main></body>
			</html>
		}
	}

	/// The sticky-title regression: through the real `wrap_content` pipeline
	/// (per-request layout + transcluded content + fresh `LayoutContent`), each
	/// route renders its *own* `<title>` (not the previous request's), the visible
	/// header stays the site title, and the shared `PackageConfig.title` is never
	/// polluted by a per-route title write-back.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn route_title_differs_per_request_and_header_stays_site_title() {
		let mut world = router_world();
		world.insert_resource(PackageConfig {
			title: "SiteName".into(),
			..default()
		});
		let root = world
			.spawn((Router, Layout::of::<MetaLayout>(), children![
				meta_route("alpha", "Alpha"),
				meta_route("beta", "Beta"),
			]))
			.flush();

		// each route renders exactly one `<title>`, carrying its own route title.
		let alpha = get(&mut world, root, "alpha").await;
		alpha.matches("<title>").count().xpect_eq(1);
		alpha.as_str().xpect_contains("<title>Alpha</title>");
		// the header link is always the site title, never the route title.
		alpha.as_str().xpect_contains("app-bar-title");
		alpha
			.split("app-bar-title")
			.nth(1)
			.unwrap()
			.xpect_contains("SiteName");

		// a different route renders a different title (not sticky on "Alpha").
		let beta = get(&mut world, root, "beta").await;
		beta.as_str().xpect_contains("<title>Beta</title>");
		beta.as_str().xnot().xpect_contains("<title>Alpha</title>");

		// re-requesting alpha is fresh again, not stuck on the last request.
		get(&mut world, root, "alpha")
			.await
			.as_str()
			.xpect_contains("<title>Alpha</title>");

		// the shared resource was never overwritten by a per-route title.
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("SiteName");
	}

	/// A page names its own url as canonical, while the not-found page, whose
	/// [`PageStatus`] is a `404`, claims no url at all rather than its own.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn error_page_claims_no_canonical() {
		let mut world = router_world();
		world.insert_resource(PackageConfig {
			homepage: Some("https://example.test".into()),
			..default()
		});
		let root = world
			.spawn((Router, Layout::of::<MetaLayout>(), children![meta_route(
				"alpha", "Alpha"
			),]))
			.flush();

		get(&mut world, root, "alpha")
			.await
			.as_str()
			.xpect_contains(
				r#"rel="canonical" href="https://example.test/alpha""#,
			)
			.xpect_contains(
				r#"property="og:url" content="https://example.test/alpha""#,
			)
			.xpect_contains(r#""url":"https://example.test/alpha""#);

		let response = world
			.entity_mut(root)
			.exchange(
				Request::get("missing")
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await;
		response.status().xpect_eq(StatusCode::NOT_FOUND);
		response
			.text()
			.await
			.unwrap()
			.as_str()
			.xpect_contains("Not found")
			.xnot()
			.xpect_contains(r#"rel="canonical""#)
			.xnot()
			.xpect_contains("og:url")
			.xnot()
			.xpect_contains(r#""url":"#);
	}

	/// The SSR seed of the `<title>` (what `RouteHead` renders before any document
	/// sync, and all a static export ever gets) reads the route's meta through a
	/// nested layout, not the intervening layout root.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn nested_layout_seeds_the_route_title() {
		let mut world = router_world();
		world.init_resource::<PackageConfig>();
		let store = meta_layout_store().await;
		let root = world
			.spawn((store, Router, Layout::of::<MetaLayout>(), children![(
				PathPartial::new("blog"),
				Layout::document("ArticleLayout.bsx"),
				children![meta_route("post", "Post")]
			)]))
			.flush();

		get(&mut world, root, "blog/post")
			.await
			.as_str()
			.xpect_contains("<title>Post</title>");
	}

	/// The layout link [`LayoutContent`] (and its reverse [`LayoutContentOf`])
	/// survives a scene serialization round-trip: a reflect-registered
	/// relationship whose `#[entities]` source remaps onto the freshly spawned
	/// content, and whose collection target the relationship hook rebuilds on
	/// load. This is the link a reloaded layout tree's reserved bindings follow.
	#[cfg(all(feature = "scene", feature = "json"))]
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
