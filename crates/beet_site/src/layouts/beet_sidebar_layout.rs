#![allow(unused)]
use crate::prelude::*;
use beet::prelude::*;


#[template]
pub fn BeetSidebarLayout(
	entity: Entity,
	world: &mut World,
) -> Result<impl Bundle> {
	let endpoint_tree = world.run_system_cached_with(
		|entity: In<Entity>,
		 bundle_query: HtmlBundleQuery,
		 mut route_query: RouteQuery|
		 -> Result<EndpointTree> {
			let actions =
				bundle_query.actions_from_agent_descendant(*entity).unwrap();
			assert_eq!(actions.len(), 1);
			route_query.endpoint_tree(actions[0])
		},
		entity,
	)?;

	let sidebar_nodes = world
		.run_system_cached_with(
			CollectSidebarNode::collect,
			(
				CollectSidebarNode {
					include_filter: GlobFilter::default()
						.with_include("/")
						.with_include("/docs*")
						.with_include("/blog*")
						.with_include("/design*"),
					expanded_filter: GlobFilter::default()
						.with_include("/docs"),
				},
				endpoint_tree,
			),
		)?
		.children;

	// let sidebar_nodes = todo!("get route tree");
	rsx! {
		<BeetContext>
		<PageLayout>
			<BeetHead slot="head"/>
		// <slot name="head" slot="head" />
		// <slot name="header" slot="header" />
		// <slot name="header-nav" slot="header-nav" />
		// <slot name="footer" slot="footer" />
		<BeetHeaderLinks slot="header-nav"/>
		<div class="container">
			<Sidebar nodes=sidebar_nodes />
			<main class="bt-u-main">
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
	.xok()
}
