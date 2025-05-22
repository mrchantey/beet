use crate::prelude::*;


/// A [`PageLayout`] with a `<main>` content area.
#[derive(derive_template)]
pub struct ContentLayout {
	// pub page_layout: PageLayout,
}

fn content_layout(_: ContentLayout) -> WebNode {
	rsx! {
			<PageLayout>
			<slot name="head" slot="head" />
			<slot name="header" slot="header" />
			<slot name="header-nav" slot="header-nav" />
			<slot name="footer" slot="footer" />
			<main>
				<slot />
			</main>
			</PageLayout>
			<style>
				main {
					/* min-height:100dvh; */
					min-height: var(--bt-main-height);
					padding: 1.em var(--bt-content-padding-width);
				}
				main img {
					max-width: 100%;
				}
			</style>
	}
}
