use crate::prelude::*;

/// Layout for any html page, superset of [`DocumentLayout`].
#[derive(Node)]
pub struct PageLayout;

fn page_layout(_props: PageLayout) -> WebNode {
	rsx! {
		<DocumentLayout>
		<slot name="head" slot="head" />
		<div class="page">
			<Header>
				<slot name="header" slot="default" />
				<slot name="header-nav" slot="nav" />
			</Header>
			<slot/>
			<Footer>
				<slot name="footer" slot="default" />
			</Footer>
		</div>
		</DocumentLayout>
		<style>
		.page {
			min-height: 100dvh;
			display: flex;
			flex-direction: column;
			/* overflow: hidden; */
		}
		</style>
	}
}
