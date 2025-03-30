use crate::prelude::*;


/// A [`PageLayout`] with a `<main>` content area.
#[derive(Node)]
pub struct ContentLayout {
	// pub page_layout: PageLayout,
}

fn content_layout(_: ContentLayout) -> RsxNode {
	rsx! {
			<PageLayout>
			<slot name="head" slot="head" />
			<main>
				<slot />
			</main>
			</PageLayout>
			<style>
				main {
					/* min-height:100dvh; */
					min-height: var(--bm-main-height);
					padding: 1.em var(--content-padding-width);
				}
				main img {
					max-width: 100%;
				}
			</style>
	}
}
