use crate::prelude::*;

/// A [`PageLayout`] with a `<main>` content area.
#[template]
pub fn ContentLayout() -> impl Bundle {
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
					padding: 1.em var(--bt-content-padding-width) 1.em var(--bt-content-padding-width);
				}
				main img {
					max-width: 100%;
				}
			</style>
	}
}
