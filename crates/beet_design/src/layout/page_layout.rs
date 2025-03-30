use crate::prelude::*;

/// Layout for any html page, superset of [`DocumentLayout`].
#[derive(Node)]
pub struct PageLayout {
	// pub document_layout: DocumentLayout,
}

fn page_layout(_props: PageLayout) -> RsxNode {
	rsx! {
		<DocumentLayout>
		<slot name="head" slot="head" />
		<div class="page">
			// <Header/>
			<slot/>
			// <Footer/>
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
