use crate::prelude::*;

/// A dynamic page rendered per request, with access to the [`Request`].
pub async fn get(_cx: ActionContext<Request>) -> impl Bundle {
	rsx! {
		<main>
			<h1>"About"</h1>
			<p>"This page is rendered per request via a scene adapter."</p>
			<a href={routes::index()}>"Home"</a>
		</main>
	}
}
