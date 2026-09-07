//! The permanent-redirect route: an old url kept alive after a page was renamed.

use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A markup-spawnable permanent redirect: `path` answers `GET` with a 301 to
/// `redirect`, so a url a page used to serve at keeps working after a rename.
///
/// The target resolves against this route's own scope — the segments its
/// ancestors contribute — unless it starts with `/`, in which case it is taken
/// verbatim. So under `<Route path="blog">`,
/// `<Redirect path="post-1" redirect="full-stack-bevy"/>` maps `/blog/post-1` to
/// `/blog/full-stack-bevy`, and the redirect block reads as a list of renames
/// rather than a list of absolute urls.
///
/// Carries no [`PageRoute`], so it stays out of the navigation
/// ([`RouteSidebar`]) and any generated [`RouteIndex`]; it is deliberately NOT
/// [`RouteHidden`], which drops a route from the [`RouteTree`] altogether and
/// so from dispatch. It leaves its [`ExportStrategy`] at the default `Dynamic`,
/// so static export skips it too: an exported site serves the redirect only
/// where the host understands one. Runtime serving (the deployed site, behind a
/// caching proxy) answers the real 301. A future static-export story should
/// emit a `<path>/index.html` meta-refresh stub with a canonical link for these.
///
/// The Rust equivalent is spawning the same bundle; like [`Route`] it is a
/// [`template`](macro@template) rather than a component, so it expands away at
/// build time with nothing left to re-fire on reload.
///
/// [`PageRoute`]: crate::prelude::PageRoute
/// [`RouteSidebar`]: crate::prelude::RouteSidebar
/// [`RouteIndex`]: crate::prelude::RouteIndex
/// [`RouteHidden`]: crate::prelude::RouteHidden
/// [`RouteTree`]: crate::prelude::RouteTree
/// [`Route`]: crate::prelude::Route
#[template]
pub fn Redirect(
	/// The route path pattern to redirect FROM, eg `post-1`.
	#[prop(into)]
	path: String,
	/// Where to send the caller: a name resolved against this route's parent
	/// scope, or an absolute url when it starts with `/`.
	#[prop(into)]
	redirect: String,
) -> impl Bundle {
	// how many segments of this route's pattern are its own, ie how many to drop
	// to reach the scope its ancestors set
	let depth = SmolPath::new(path.as_str()).segments().len();
	(
		PathPartial::new(path),
		HttpMethod::Get,
		Action::<Request, Response>::new_async(
			async move |cx: ActionContext<Request>| -> Result<Response> {
				let location = match redirect.starts_with('/') {
					true => redirect.clone(),
					// the scope is only known once the tree is built, so it is read
					// off the route's own resolved pattern at dispatch
					false => cx
						.caller
						.get(|pattern: &PathPattern| pattern.annotated_path())
						.await?
						.xmap(|pattern| {
							let mut segments = pattern.segments();
							segments
								.truncate(segments.len().saturating_sub(depth));
							SmolPath::from_segments(&segments)
						})
						.join(redirect.as_str())
						.with_leading_slash(),
				};
				Response::permanent_redirect(location).xok()
			},
		),
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use bevy::ecs::error::ErrorContext;
	use bevy::ecs::error::FallbackErrorHandler;
	use std::sync::Mutex;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// The `Location` of a `GET` to `path` through `root`, asserting a 301.
	async fn location(world: &mut World, root: Entity, path: &str) -> String {
		let res = world.entity_mut(root).exchange(Request::get(path)).await;
		res.status().xpect_eq(StatusCode::MOVED_PERMANENTLY);
		res.parts
			.headers
			.get::<header::Location>()
			.unwrap()
			.unwrap()
			.to_string()
	}

	/// A renamed page's old url answers a 301 at the new one, resolved against
	/// the prefix its ancestors set rather than an absolute url authored here.
	#[beet_core::test]
	async fn redirects_within_parent_scope() {
		let mut world = router_world();
		let root = world.spawn(Router).flush();
		// the routes build through the template substrate (so `<Route>`'s slot
		// resolves) directly into their place under the router
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Route path="blog">
					<Redirect path="post-1" redirect="full-stack-bevy"/>
					<Redirect path="post-2" redirect="/elsewhere"/>
				</Route>
			})))
			.unwrap();
		world.flush();

		location(&mut world, root, "blog/post-1")
			.await
			.xpect_eq("/blog/full-stack-bevy");
		// a leading `/` opts out of the scope entirely
		location(&mut world, root, "blog/post-2")
			.await
			.xpect_eq("/elsewhere");
	}

	/// A redirect is not a page: it stays out of the navigation and out of a
	/// generated [`RouteIndex`](crate::prelude::RouteIndex), while remaining dispatchable (which a
	/// [`RouteHidden`](crate::prelude::RouteHidden) redirect would not be, being dropped from the tree).
	#[beet_core::test]
	async fn is_not_a_page_route() {
		let mut world = router_world();
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Redirect path="post-1" redirect="renamed"/>
			})))
			.unwrap();
		world.flush();

		let node = world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["post-1"])
			.unwrap();
		node.is_page_route.xpect_false();
		location(&mut world, root, "post-1")
			.await
			.xpect_eq("/renamed");
	}

	/// A bare `<Route>` prefix and a same-named page route merge into one tree
	/// node, so a redirect block sits beside the pages it points at without
	/// tripping duplicate-path validation.
	#[beet_core::test]
	async fn prefix_merges_with_existing_subtree() {
		let mut world = router_world();
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Route path="blog">
					<Redirect path="post-1" redirect="full-stack-bevy"/>
				</Route>
			})))
			.unwrap();
		world.flush();
		// the page the prefix collides with, discovered separately (as a
		// `<RoutesDir>` scan would spawn it)
		world.spawn((
			ChildOf(root),
			render_action::fixed_func_route("blog", || rsx! { <p>"index"</p> }),
			PageRoute,
		));
		world.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// one `blog` node, carrying the index route and the redirect child
		tree.find(&["blog"]).xpect_some();
		tree.find(&["blog", "post-1"]).xpect_some();
		location(&mut world, root, "blog/post-1")
			.await
			.xpect_eq("/blog/full-stack-bevy");
	}

	/// A redirect is an ordinary route, so one colliding with a real page raises
	/// the duplicate-path error every route pair does
	/// ([`RouteTree::from_nodes`]), rather than one of the two silently winning.
	#[beet_core::test]
	fn collision_with_real_route_errors() {
		let mut world = router_world();
		// the rebuild raises through the command error handler, which panics by
		// default; swap in the recorder so the message can be asserted.
		*LAST_ERROR.lock().unwrap() = None;
		world.insert_resource(FallbackErrorHandler(record_error));
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Redirect path="post-1" redirect="renamed"/>
			})))
			.unwrap();
		world.spawn((
			ChildOf(root),
			render_action::fixed_func_route(
				"post-1",
				|| rsx! { <p>"page"</p> },
			),
			PageRoute,
		));
		world.flush();
		LAST_ERROR
			.lock()
			.unwrap()
			.clone()
			.unwrap()
			.xpect_contains("Duplicate route");
	}

	/// The message the [`collision_with_real_route_errors`] handler swallowed. A
	/// [`FallbackErrorHandler`] is a bare fn pointer with nowhere to capture, so
	/// it lands here.
	static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

	/// Records a raised command error instead of panicking on it.
	fn record_error(err: BevyError, _: ErrorContext) {
		*LAST_ERROR.lock().unwrap() = Some(err.to_string());
	}
}
