use crate::prelude::*;
use beet::prelude::*;


#[template]
pub fn BeetSidebarLayout() -> impl Bundle {
	let sidebar_nodes =
		route_path_tree().xpipe(RoutePathTreeToSidebarTree::default());
		beet::log!("sidebar_nodes: {sidebar_nodes:#?}");
	rsx! {
		<BeetContext>
		<PageLayout>
		// <slot name="head" slot="head" />
		// <slot name="header" slot="header" />
		// <slot name="header-nav" slot="header-nav" />
		// <slot name="footer" slot="footer" />
		<BeetHeaderLinks slot="header-nav"/>
		<div class="container">
			<Sidebar nodes=sidebar_nodes />
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
			padding: 1.em calc(max((100% - 100rem) * 0.5, 1.em));
			/* padding: 0 10em 0 2em; */
			/* padding: 0 calc(var(--content-padding-width) - var(--sidebar-width)) 0 0; */
		}
		</style>
	}
}
