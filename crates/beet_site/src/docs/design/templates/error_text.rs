use beet::prelude::*;

/// Shows the [`ErrorText`] widget used for inline validation feedback.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Error Text"</h1>
			<p>"Validation feedback rendered in the error color:"</p>
			<ErrorText message="This is an error"/>
			<p>
				"Callers conditionally include the widget, so an absent error renders nothing."
			</p>
		</article>
	}
}
