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
/// since there is no response to render: a static export instead writes a
/// meta-refresh stub for it (see [`StaticExport`]), while runtime serving
/// answers the real 301.
///
/// The Rust equivalent is spawning the same bundle; like [`Route`] it is a
/// [`template`](macro@template) rather than a component, so it expands away at
/// build time with nothing left to re-fire on reload. What it leaves behind is
/// [`RedirectTo`], the target as plain data, which both dispatch and the export
/// read.
///
/// [`PageRoute`]: crate::prelude::PageRoute
/// [`RouteSidebar`]: crate::prelude::RouteSidebar
/// [`RouteIndex`]: crate::prelude::RouteIndex
/// [`RouteHidden`]: crate::prelude::RouteHidden
/// [`RouteTree`]: crate::prelude::RouteTree
/// [`Route`]: crate::prelude::Route
/// [`StaticExport`]: crate::prelude::StaticExport
#[template]
pub fn Redirect(
	/// The route path pattern to redirect FROM, eg `post-1`.
	#[prop]
	path: String,
	/// Where to send the caller: a name resolved against this route's parent
	/// scope, a rooted path taken verbatim, or another site entirely.
	#[prop]
	redirect: Url,
) -> impl Bundle {
	(
		RedirectTo::new(&path, redirect),
		PathPartial::new(path),
		HttpMethod::Get,
		Action::<Request, Response>::new_async(
			async move |cx: ActionContext<Request>| -> Result<Response> {
				let location = cx
					.caller
					.with_state::<Query<(&RedirectTo, &PathPattern)>, _>(
						|entity, query| {
							query
								.get(entity)
								.map(|(redirect, pattern)| {
									redirect.location(pattern)
								})
								.map_err(BevyError::from)
						},
					)
					.await??;
				Response::permanent_redirect(location.to_string()).xok()
			},
		),
	)
}

/// Where a [`Redirect`] route sends a caller, as plain data on the route
/// entity.
///
/// A component rather than a value captured in the handler because two
/// consumers read it: dispatch answers the 301 with it, and a static export
/// writes a meta-refresh stub from it for hosts that serve files and nothing
/// else.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct RedirectTo {
	/// The authored target. A [rooted](Url::is_rooted) path or another origin
	/// is taken verbatim; anything else is a relative reference resolved
	/// against this route's parent scope.
	pub target: Url,
	/// How many segments of this route's own pattern are its own, ie how many
	/// to drop to reach the scope its ancestors set. Resolved from the authored
	/// `path` at build, since the full pattern only exists once the tree is
	/// built.
	depth: usize,
}

impl RedirectTo {
	/// The target of a route declaring `path`, eg
	/// `RedirectTo::new("post-1", "full-stack-bevy")`.
	pub fn new(path: &str, target: impl Into<Url>) -> Self {
		Self {
			target: target.into(),
			depth: SmolPath::new(path).segments().len(),
		}
	}

	/// The `Location` this route sends a caller to, resolved against the full
	/// route `pattern` its ancestors gave it.
	///
	/// A target naming another origin, or a rooted one, is already a complete
	/// destination and passes through; a relative one is resolved against the
	/// scope this route's ancestors set, which is what lets a redirect block
	/// read as a list of renames rather than a list of absolute urls.
	pub fn location(&self, pattern: &PathPattern) -> Url {
		if self.target.is_external() || self.target.is_rooted() {
			return self.target.clone();
		}
		let path = pattern.annotated_path();
		let mut segments = path.segments();
		segments.truncate(segments.len().saturating_sub(self.depth));
		SmolPath::from_segments(&segments)
			.xmap(Url::from)
			.with_rooted(true)
			// the scope is a directory, so the reference resolves beside a
			// trailing segment that is not there: push an empty one for `join`
			// to drop, exactly as a browser resolves `href` against `/blog/`
			.push("")
			.join(self.target.clone())
	}
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
	/// A rooted target, or one naming another origin, is already a complete
	/// destination and passes through untouched.
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
					<Redirect path="post-3" redirect="https://example.com/moved"/>
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
		// ..and so does another origin, which the scope could not prefix
		// meaningfully anyway
		location(&mut world, root, "blog/post-3")
			.await
			.xpect_eq("https://example.com/moved");
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
