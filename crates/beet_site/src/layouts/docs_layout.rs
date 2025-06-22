use crate::prelude::*;
use beet::prelude::*;

#[template]
pub fn DocsLayout(meta: DocsMeta) -> impl Bundle {
	rsx! {
		<BeetSidebarLayout>
		<h1>{meta.title.unwrap_or("File".to_string())}</h1>
		<slot/>
		</BeetSidebarLayout>
	}
}
