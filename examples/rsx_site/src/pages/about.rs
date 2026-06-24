use crate::prelude::*;
use beet::prelude::*;

/// A dynamic page rendered per request, with access to the [`Request`].
///
/// Where the other pages are sync `fn get() -> impl Bundle` (the
/// `fixed_func_route` codegen branch), this `async fn get(ActionContext<Request>)`
/// drives the `async_route` branch: the body is built per request, so it can read
/// the live request (here, the negotiated path).
pub async fn get(cx: ActionContext<Request>) -> impl Bundle {
	let path = cx.input.path_string();
	rsx! {
		<article>
			<h1>"About"</h1>
			<p>
				"This page is rendered per request, with access to the live "
				<code>"Request"</code>". The Rust counterpart of a no-code dynamic route."
			</p>
			<p>"You reached it at "<code>{path}</code>"."</p>
			<Link href=routes::guide() variant=ButtonVariant::Text>"Read the guide"</Link>
		</article>
	}
}
