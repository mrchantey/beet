use crate::prelude::*;


/// A [`PageLayout`] with a `<main>` content area.
#[derive(Node)]
pub struct ContentLayout {
	// pub page_layout: PageLayout,
}

fn content_layout(_: ContentLayout) -> RsxRoot {
	rsx! {
			<PageLayout>
			<main>
				<slot />
			</main>
			</PageLayout>
			<style>
				// main {
				// 	/* min-height:100dvh; */
				// 	min-height: var(--bm-main-height);
				// 	padding: 1em var(--content-padding-width);
				// }
				// main img {
				// 	max-width: 100%;
				// }
			</style>
	}
}
