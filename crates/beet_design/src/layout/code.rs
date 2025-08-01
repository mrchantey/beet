use crate::prelude::*;

#[template]
pub fn Code(
	#[field(flatten)] attrs: BaseHtmlAttributes,
	// content: String,
) -> impl Bundle {
	rsx! {
		<pre {attrs}>
		<slot/>
		</pre>
		<style>
			pre{
				display: flex;
				flex-direction: column;
				border-radius: var(--bt-border-radius);
				color: var(--bt-color-on-surface);
				background-color: var(--bt-color-surface-container);
			}
		</style>
	}
}
