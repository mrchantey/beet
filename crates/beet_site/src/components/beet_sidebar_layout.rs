use crate::prelude::*;
use beet::prelude::*;


#[derive(Node)]
pub struct BeetSidebarLayout {}


fn beet_sidebar_layout(_: BeetSidebarLayout) -> RsxNode {
	let routes = route_tree::collect_static_route_tree();
	rsx! {
		<BeetContext>
		<PageLayout>
		<slot name="head" slot="head" />
		<slot name="header" slot="header" />
		<slot name="header-nav" slot="header-nav" />
		<slot name="footer" slot="footer" />
		// <BeetHeader
		<a class="bt-u-button-like" href="/" slot="header-nav">click me</a>
		<div class="container">
			<Sidebar routes={routes} />
			<main>
				<slot/>
			</main>
		</div>
		</PageLayout>
		</BeetContext>
		<style>
		.container{
	
			--sidebar-width:20rem;
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
