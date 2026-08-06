//! The static route serving the thin-client reactivity runtime.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::Reactivity;

/// A [`ExportStrategy::Static`] route serving the thin-client reactivity runtime
/// ([`Reactivity::JS`]) as `application/javascript` at [`Reactivity::SRC`].
///
/// `Router::with_defaults` wires it in, so a served page's auto-injected
/// `<script defer src="/js/reactivity.js">` resolves, and `export-static` emits
/// it as a file (a statically exported reactive site is self-contained).
pub(crate) fn reactivity_js_route() -> impl Bundle {
	(
		// the route path is the src URL without its leading slash
		Router::exchange_route(
			Reactivity::SRC.trim_start_matches('/'),
			exchange_ext::handler(|_: ActionContext<Request>| {
				Response::ok_body(Reactivity::JS, MediaType::Javascript)
			}),
		),
		HttpMethod::Get,
		ExportStrategy::Static,
		// compiled-in bytes, fetched on every page view: edge-cacheable, browsers
		// revalidate (a deploy purge is what changes it)
		CacheHeaders::any_media(),
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn serves_the_runtime() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world
			.spawn(Router::with_defaults())
			.exchange(Request::get("js/reactivity.js"))
			.await
			.unwrap_str()
			.await
			// the real runtime, not a stub
			.xpect_contains("class EntityMut")
			.xpect_contains("globalThis.beet");
	}
}
