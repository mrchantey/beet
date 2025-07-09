use crate::prelude::*;

#[template]
pub fn Card(#[field(flatten)] attrs: BaseHtmlAttributes) -> impl Bundle {
	rsx! {
		<article {attrs}>
			<slot/>
		</article>
		<style>
			article{
				display: flex;
				flex-direction: column;
				gap: var(--bt-spacing);
				padding: var(--bt-spacing);
				border-radius: var(--bt-border-radius);
				color: var(--bt-color-on-surface);
				background-color: var(--bt-color-surface-container);
			}
		</style>
	}
}
