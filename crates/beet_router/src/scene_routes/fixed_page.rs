//! The persistent page route: one live tree, served request after request.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marks a [`Route`] whose declared children *are* the page: one live tree,
/// built once and served by every request, rather than rebuilt per request.
///
/// Every other page route ([`render_action`], [`BlobScene`]) spawns a fresh tree
/// per request and despawns it after render. A `FixedPage` instead makes the
/// route entity its own [`PageRoot`] with no [`DespawnAfterRender`], so the tree
/// its children form outlives the response:
///
/// ```bsx
/// <Route path="/" {FixedPage}>
///     <ThreadView {OfThread($thread)}/>
/// </Route>
/// ```
///
/// Persistence is what makes such a page *reactive*: a system writing into the
/// live tree (a thread projection, a document sync) is seen by the next render,
/// and a `$thread`-style entity reference resolves once at entry build rather
/// than per request. An ancestor layout still wraps it per request, transcluding
/// the fixed tree by reference (see [`Layout`]).
///
/// # One surface only
///
/// This is a *live surface* addressed by a route, not a page in the usual sense,
/// and the distinction is a real limitation. The tree is per route (three
/// `FixedPage` routes over three threads each own their own) but **shared by
/// every surface viewing it**, widget state included: two terminals on one route
/// share the same `<input>`, scroll offset and focus. A per-request route avoids
/// this by construction — [`BlobScene`] reparses its document per request for
/// exactly that reason.
///
/// It earns its keep for the single local terminal, where the surface repaints
/// continuously rather than answering requests, so the tree *must* outlive any
/// one response: the projection systems write into it between frames, and a
/// `$ref` entity reference resolves once at entry build and cannot be re-resolved
/// per request. Concurrent viewers are the case it does not serve.
///
/// The fix is per-surface instances over one shared model, which needs a way to
/// name an entity across a build boundary (a lookup, not a build-scoped `$ref`) —
/// the same primitive a separate page file would need, and squarely what the BSN
/// asset format will redefine. Re-open it there, or sooner if a second concurrent
/// surface is wanted.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(FixedPageAction)]
#[component(on_add = hook_ext::entity_hook(FixedPage::insert_page_root))]
pub struct FixedPage;

impl FixedPage {
	/// Make the route its own render root with nothing to clean up after render,
	/// the one difference from a per-request page.
	///
	/// Runs on add rather than per request so the tree is a [`RouteQuery`]
	/// boundary from the moment it is spawned, and so a render never mutates the
	/// route to serve it.
	fn insert_page_root(entity: &mut EntityCommands) {
		let id = entity.id();
		entity.insert((PageRootOf(id), DespawnAfterRender::default()));
	}
}

/// The `Request -> PageRequest` route action [`FixedPage`] installs: hand back
/// the route entity itself as the render root, whatever the request.
#[action(route)]
#[derive(Default, Component)]
async fn FixedPageAction(cx: ActionContext<Request>) -> Result<PageRequest> {
	PageRequest(cx.id()).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A router serving one `FixedPage` route whose page is a `<p>`. Returns the
	/// router root.
	fn spawn_router(world: &mut World) -> Entity {
		world
			.spawn((Router, children![(route::new("", FixedPage), children![
				rsx! { <p>"page body"</p> }
			])]))
			.flush()
	}

	async fn get(world: &mut World, root: Entity, path: &str) -> String {
		world
			.entity_mut(root)
			.exchange(Request::get(path))
			.await
			.unwrap_str()
			.await
	}

	/// The one tree serves every request: two exchanges render the same body and
	/// the entity count is unchanged, where a per-request route would rebuild.
	#[beet_core::test]
	async fn persists_across_requests() {
		let mut world = router_world();
		let root = spawn_router(&mut world);

		get(&mut world, root, "").await.xpect_contains("page body");
		let baseline = world.iter_entities().count();
		get(&mut world, root, "").await.xpect_contains("page body");
		world.iter_entities().count().xpect_eq(baseline);
	}

	/// The rendered tree is the *live* one: a write into the page between two
	/// requests shows up in the second, which is the whole point of a fixed page.
	#[beet_core::test]
	async fn renders_the_live_tree() {
		let mut world = router_world();
		let root = spawn_router(&mut world);
		get(&mut world, root, "").await.xpect_contains("page body");

		// mutate the live `<p>`'s text node in place
		let text = world
			.query_once::<(Entity, &Value)>()
			.into_iter()
			.find(|(_, value)| value.as_str().ok() == Some("page body"))
			.map(|(entity, _)| entity)
			.unwrap();
		world.entity_mut(text).insert(Value::new("edited body"));

		get(&mut world, root, "")
			.await
			.as_str()
			.xpect_contains("edited body")
			.xnot()
			.xpect_contains("page body");
	}

	/// The markup form: a `<Route>`'s declared children fill its slot, which
	/// collapses, so they end up direct children of the route entity and render
	/// as the page. Several children need no wrapper.
	#[beet_core::test]
	async fn serves_route_template_children() {
		let mut world = router_world();
		let root = world.spawn(Router).flush();
		// the route builds through the template substrate (so `<Route>`'s slot
		// resolves) directly into its place under the router
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Route path="" {FixedPage}>
					<p>"first"</p>
					<p>"second"</p>
				</Route>
			})))
			.unwrap();

		get(&mut world, root, "")
			.await
			.as_str()
			.xpect_contains("first")
			.xpect_contains("second");
	}

	/// A layout wraps the fixed page per request, transcluding the shared tree by
	/// reference: the layout is ephemeral, the page survives it.
	#[beet_core::test]
	async fn survives_its_layout() {
		#[template]
		fn FixedShell() -> impl Bundle {
			rsx! { <html><body><main><Slot/></main></body></html> }
		}
		let mut world = router_world();
		world.register_template::<FixedShell>();
		let root = world
			.spawn((Router, Layout::of::<FixedShell>(), children![(
				route::new("", FixedPage),
				children![rsx! { <p>"page body"</p> }]
			)]))
			.flush();

		let first = get(&mut world, root, "").await;
		let second = get(&mut world, root, "").await;
		second.as_str().xpect_contains("<p>page body</p>");
		first.xpect_eq(second);
	}
}
