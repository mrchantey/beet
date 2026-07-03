use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The `Cache-Control` policy for a route or subtree, written by
/// [`CacheHeadersMiddleware`]: the counterpart of [`NoCacheHeaders`] for a
/// site behind a purgeable edge cache (a CDN).
///
/// Pure config, resolved from the handled route by nearest self-or-ancestor
/// (like [`AnalyticsConfig`] gates [`AnalyticsMiddleware`]): add one to the
/// router for a site-wide default, and to any route or subtree for a more
/// specific policy. Config-not-action matters: a route entity carries its
/// handler's [`ActionMeta`], which a second action component would clobber.
///
/// By default only `text/html` responses are marked cacheable and every other
/// media type gets `no-store`: routes content-negotiate on `Accept` (a page
/// serves html to a browser and markdown to a terminal from the same URL), and
/// most edges key their cache on the URL alone, so a cached non-html variant
/// would answer browser requests too. Single-media routes (static files) opt
/// out with [`media_agnostic`](Self::media_agnostic).
///
/// Browsers cannot be purged, so [`browser_max_age`](Self::browser_max_age)
/// stays short and the edge revalidates on their behalf; the purge on deploy /
/// content sync is what refreshes [`edge_max_age`](Self::edge_max_age)-held
/// responses early.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CacheHeaders {
	/// How long a shared cache (the CDN edge) may hold the response
	/// (`s-maxage`). Long by default: the deploy/sync purge refreshes earlier.
	pub edge_max_age: Duration,
	/// How long a browser may reuse the response without revalidating
	/// (`max-age`). Zero by default: browsers cannot be purged.
	pub browser_max_age: Duration,
	/// Apply the policy to every media type rather than only `text/html`.
	pub media_agnostic: bool,
	/// Emit the [`NoCacheHeaders`] value instead: the route-level opt-out for
	/// live responses (diagnostics, upgrade handshakes) under a cacheable
	/// default.
	pub no_store: bool,
}

impl Default for CacheHeaders {
	/// The site-wide default: html edge-cacheable, browsers revalidate, every
	/// other media type `no-store`.
	fn default() -> Self {
		Self {
			edge_max_age: Duration::from_secs(7 * 24 * 60 * 60),
			browser_max_age: Duration::ZERO,
			media_agnostic: false,
			no_store: false,
		}
	}
}

impl CacheHeaders {
	/// The single-media policy: every media type edge-cacheable, browsers
	/// revalidate. For routes that never content-negotiate (a compiled-in
	/// script, a single-format API).
	pub fn any_media() -> Self {
		Self {
			media_agnostic: true,
			..default()
		}
	}

	/// The static-file policy: every media type cacheable, browsers may hold
	/// for an hour (files are not content-hashed, so browser TTL stays modest).
	pub fn assets() -> Self {
		Self {
			browser_max_age: Duration::from_secs(60 * 60),
			media_agnostic: true,
			..default()
		}
	}

	/// The live-response policy: never stored anywhere.
	pub fn no_store() -> Self {
		Self {
			no_store: true,
			..default()
		}
	}

	/// The `Cache-Control` value for a response of `media_type` under this
	/// policy.
	fn value(&self, media_type: Option<MediaType>) -> String {
		if self.no_store {
			return "no-cache, no-store, must-revalidate".to_string();
		}
		if !self.media_agnostic && media_type != Some(MediaType::Html) {
			// a non-html negotiated variant (markdown for terminals) must never
			// be edge-cached where a browser could receive it
			return "no-store".to_string();
		}
		let browser_secs = self.browser_max_age.as_secs();
		// browsers holding nothing must revalidate rather than heuristically cache
		let revalidate = if browser_secs == 0 {
			", must-revalidate"
		} else {
			""
		};
		format!(
			"public, max-age={browser_secs}{revalidate}, s-maxage={}",
			self.edge_max_age.as_secs()
		)
	}
}

/// Middleware writing `Cache-Control` per the handled route's nearest
/// self-or-ancestor [`CacheHeaders`]: attach once at the router (alongside the
/// site-wide [`CacheHeaders`] default), the way [`AnalyticsMiddleware`] pairs
/// with [`AnalyticsConfig`]. Without a resolved policy it is a pass-through,
/// so the pair is opt-in.
///
/// Set-if-absent: a response that already carries `Cache-Control` (an explicit
/// [`NoCacheHeaders`] layer, a handler-set value) is left untouched.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn CacheHeadersMiddleware(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();
	let mut response = next.call(request).await?;
	let headers = &mut response.parts.headers;
	// set-if-absent: an explicit header wins
	if headers.get::<header::CacheControl>().is_some() {
		return Ok(response);
	}
	// the handled route's nearest policy; none resolved is a pass-through
	let Some(policy) = caller
		.with_state::<AncestorQuery<&CacheHeaders>, Option<CacheHeaders>>(
			|entity, query| query.get(entity).ok().cloned(),
		)
		.await?
	else {
		return Ok(response);
	};
	let media_type = headers
		.get::<header::ContentType>()
		.and_then(|result| result.ok());
	headers.set::<header::CacheControl>(policy.value(media_type));
	Ok(response)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn HelloHtml(_cx: ActionContext<RequestParts>) -> Response {
		Response::ok_body("<p>Hello</p>", MediaType::Html)
	}

	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn HelloMarkdown(_cx: ActionContext<RequestParts>) -> Response {
		Response::ok_body("# Hello", MediaType::Markdown)
	}

	async fn cache_control(
		world: &mut World,
		root: Entity,
		path: &str,
	) -> String {
		world
			.entity_mut(root)
			.exchange(Request::get(path))
			.await
			.headers
			.get::<header::CacheControl>()
			.unwrap()
			.unwrap()
	}

	/// The site-wide pair: the middleware + the html-only default policy.
	fn caching_router() -> impl Bundle {
		(
			default_router(),
			CacheHeaders::default(),
			CacheHeadersMiddleware::default(),
		)
	}

	/// A router-level default marks html cacheable and other media `no-store`.
	#[beet_core::test]
	async fn html_only_default() {
		let mut world = router_world();
		let root = world
			.spawn((caching_router(), children![
				exchange_route("page", HelloHtml),
				exchange_route("page.md", HelloMarkdown),
			]))
			.flush();
		cache_control(&mut world, root, "page")
			.await
			.as_str()
			.xpect_eq("public, max-age=0, must-revalidate, s-maxage=604800");
		cache_control(&mut world, root, "page.md")
			.await
			.as_str()
			.xpect_eq("no-store");
	}

	/// The assets policy applies to every media type with a browser TTL.
	#[beet_core::test]
	async fn assets_policy() {
		let mut world = router_world();
		let root = world
			.spawn((
				default_router(),
				CacheHeaders::assets(),
				CacheHeadersMiddleware::default(),
				children![exchange_route("style", HelloMarkdown)],
			))
			.flush();
		cache_control(&mut world, root, "style")
			.await
			.as_str()
			.xpect_eq("public, max-age=3600, s-maxage=604800");
	}

	/// A route-level policy shadows the router default: nearest wins.
	#[beet_core::test]
	async fn route_policy_shadows_default() {
		let mut world = router_world();
		let root = world
			.spawn((caching_router(), children![
				(
					exchange_route("file", HelloMarkdown),
					CacheHeaders::assets()
				),
				(exchange_route("live", HelloHtml), CacheHeaders::no_store()),
			]))
			.flush();
		cache_control(&mut world, root, "file")
			.await
			.as_str()
			.xpect_eq("public, max-age=3600, s-maxage=604800");
		cache_control(&mut world, root, "live")
			.await
			.as_str()
			.xpect_eq("no-cache, no-store, must-revalidate");
	}

	/// Without a resolved policy the middleware is a pass-through: no header.
	#[beet_core::test]
	async fn no_policy_is_passthrough() {
		let mut world = router_world();
		let root = world
			.spawn((
				default_router(),
				CacheHeadersMiddleware::default(),
				children![exchange_route("page", HelloHtml)],
			))
			.flush();
		world
			.entity_mut(root)
			.exchange(Request::get("page"))
			.await
			.headers
			.get::<header::CacheControl>()
			.xpect_none();
	}
}
