use beet_core::web::prelude::html_escape;

use crate::prelude::*;

#[template]
pub fn Code(
	#[field(flatten)] attrs: BaseHtmlAttributes,
	content: String,
) -> impl Bundle {
	let content = html_escape::encode_text(&content).to_string();

	rsx! {
		<pre {attrs}>
		<code>
			{content}
			<slot/>
		</code>
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
