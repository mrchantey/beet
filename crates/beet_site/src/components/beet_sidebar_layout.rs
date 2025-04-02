use crate::prelude::*;
use beet::prelude::*;


#[derive(Node)]
pub struct BeetSidebarLayout {}


fn beet_sidebar_layout(_: BeetSidebarLayout) -> RsxNode {
	let sidebar_nodes = route_tree::collect_static_route_tree()
		.bpipe(StaticRouteTreeToSidebarTree::default());
	rsx! {
		<BeetContext>
		<PageLayout>
		// <slot name="head" slot="head" />
		// <slot name="header" slot="header" />
		// <slot name="header-nav" slot="header-nav" />
		// <slot name="footer" slot="footer" />
		<BeetHeaderLinks slot="header-nav"/>
		<div class="container">
			<Sidebar nodes={sidebar_nodes} />
			<main>
				<slot/>
			</main>
		</div>
		</PageLayout>
		</BeetContext>
		<style>
		.container{
			/* --sidebar-content-padding-width: calc(var(--content-padding-width) + 10rem); */
			min-height:var(--bt-main-height);
			display:flex;
			flex-direction: row;
		}
		main{
			width:100%;
			padding: 0 calc(max((100% - 100rem) * 0.5, 1.em));
			/* padding: 0 10em 0 2em; */
			/* padding: 0 calc(var(--content-padding-width) - var(--sidebar-width)) 0 0; */
		}
		</style>
	}
}
