//! The static route serving the thin-client reactivity runtime.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::REACTIVITY_JS;
use beet_ui::prelude::REACTIVITY_SRC;

/// A [`CacheStrategy::Static`] route serving the thin-client reactivity runtime
/// ([`REACTIVITY_JS`]) as `application/javascript` at [`REACTIVITY_SRC`].
///
/// `default_router` wires it in, so a served page's auto-injected
/// `<script defer src="/js/reactivity.js">` resolves, and `export-static` emits
/// it as a file (a statically exported reactive site is self-contained).
pub fn reactivity_js_route() -> impl Bundle {
	(
		// the route path is the src URL without its leading slash
		exchange_route(
			REACTIVITY_SRC.trim_start_matches('/'),
			exchange_handler(|_: ActionContext<Request>| {
				Response::ok_body(REACTIVITY_JS, MediaType::Javascript)
			}),
		),
		HttpMethod::Get,
		CacheStrategy::Static,
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn serves_the_runtime() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world
			.spawn(default_router())
			.call::<Request, Response>(Request::get("js/reactivity.js"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			// the real runtime, not a stub
			.xpect_contains("class EntityMut")
			.xpect_contains("globalThis.beet");
	}
}
