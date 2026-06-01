use beet::prelude::*;

/// The site navigation sidebar, collected from the router's [`RouteTree`].
///
/// Reads the single router-root [`RouteTree`] at scene build, drops the infra
/// routes (`app-info`/`analytics`), forces the `docs`/`blog` branches open, and
/// feeds the collected [`SidebarNode`] list to the [`Sidebar`] widget. The
/// active-link/current-path state is request-scoped and not yet wired here.
#[scene(system)]
pub fn BeetSidebar(trees: Query<&RouteTree, With<Router>>) -> impl Scene {
	let nodes = trees
		.iter()
		.next()
		.map(|tree| {
			SidebarState::new("")
				.with_exclude("app-info")
				.with_exclude("analytics")
				.with_info("docs", SidebarInfo {
					expanded: Some(true),
					..default()
				})
				.with_info("blog", SidebarInfo {
					expanded: Some(true),
					..default()
				})
				.collect(tree)
		})
		.unwrap_or_default();
	rsx! { <Sidebar nodes=nodes/> }
}
