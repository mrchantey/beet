use beet::prelude::*;

/// The site navigation sidebar, collected from the router's [`RouteTree`].
///
/// Reads the single router-root [`RouteTree`] at scene build and drops the
/// infra routes (`app-info`/`analytics`). The current path comes from the
/// [`RouteContext`], so the active route is marked and its ancestor branches
/// auto-expand (no forced `expanded` overrides needed).
#[scene(system)]
pub fn BeetSidebar(
	cx: &RouteContext,
	trees: Query<&RouteTree, With<Router>>,
) -> impl Scene {
	let nodes = trees
		.iter()
		.next()
		.map(|tree| {
			SidebarState::new(cx.current_path())
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
