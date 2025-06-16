use crate::prelude::*;

/// Layout for any html page, superset of [`DocumentLayout`].
#[derive(derive_template)]
pub struct PageLayout;

fn page_layout(_props: PageLayout) -> impl Bundle {
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
