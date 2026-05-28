//! `Analytics` widget — a `<script>` tag injecting [`ANALYTICS_JS`].
//!
//! Web-only: on non-web targets the `<script>` element is rendered but ignored
//! by the runtime. Gated behind the `net` feature, which provides
//! [`ANALYTICS_JS`].
use beet_core::prelude::*;
use beet_net::prelude::ANALYTICS_JS;

/// Injects the static analytics JS as a `<script>` element.
#[scene]
pub fn Analytics() -> impl Scene {
	let body = ANALYTICS_JS.to_string();
	rsx! {
		<script>{body}</script>
	}
}
