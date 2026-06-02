use beet::prelude::*;

/// Docs page embedding the `beet_action` README.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Behavior"</h1>
			<pre>{include_str!("../../../../beet_action/README.md")}</pre>
		</article>
	}
}
