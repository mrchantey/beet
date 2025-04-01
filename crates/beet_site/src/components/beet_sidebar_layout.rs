use crate::prelude::*;
use beet::prelude::*;


#[derive(Node)]
pub struct BeetSidebarLayout {}


fn beet_sidebar_layout(_: BeetSidebarLayout) -> RsxNode {
	let routes = route_tree::collect_static_route_tree();
	rsx! {
		<BeetContext>
		<ContentLayout>
		<Sidebar routes={routes} />
			<a class="bt-u-button-like" href="/" slot="header-nav">click me</a>
		<slot/>
		</ContentLayout>
		</BeetContext>
	}
}
