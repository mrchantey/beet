use crate::prelude::*;

/// The site navigation sidebar, collected from the router's [`RouteTree`].
///
/// Reads the single router-root [`RouteTree`] at scene build and drops the
/// infra routes (`app-info`/`analytics`). The current path comes from the
/// [`RequestContext`], so the active route is marked and its ancestor branches
/// auto-expand. The `docs` and `blog` collections are forced open so their
/// entries are always visible.
#[scene(system)]
pub fn BeetSidebar(
	cx: Res<RequestContext>,
	trees: Query<&RouteTree, With<Router>>,
) -> impl Scene {
	let nodes = trees
		.iter()
		.next()
		.map(|tree| {
			SidebarState::new(cx.current_path())
				.with_home(false)
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
