use crate::prelude::*;
use beet::prelude::*;

#[template]
pub fn DocsLayout(meta: DocsMeta) -> impl Bundle {
	let _ = meta;
	rsx! {
		<BeetSidebarLayout>
		<slot/>
		</BeetSidebarLayout>
	}
}
