//! Directional navigation within a [`RouteTree`].
//!
//! The [`NavigateHandler`] middleware checks for a `--navigate` param
//! and resolves the target path relative to the current request path,
//! then calls the target action directly for rendering. If the param
//! is absent, calls the inner handler via [`Next`].

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The direction to navigate relative to the current path.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum NavigateTo {
	/// Move to the parent route or root.
	#[default]
	Parent,
	/// Move to the first child route.
	FirstChild,
	/// Move to the next sibling route.
	NextSibling,
	/// Move to the previous sibling route.
	PrevSibling,
}

impl NavigateTo {
	/// Parse a CLI-style string into a [`NavigateTo`] variant.
	///
	/// Accepts kebab-case values: `parent`, `first-child`,
	/// `next-sibling`, `prev-sibling`.
	pub fn from_str_param(value: &str) -> Result<Self> {
		match value {
			"parent" => Ok(Self::Parent),
			"first-child" => Ok(Self::FirstChild),
			"next-sibling" => Ok(Self::NextSibling),
			"prev-sibling" => Ok(Self::PrevSibling),
			other => bevybail!(
				"Unknown navigate direction '{}'. \
				 Expected: parent, first-child, next-sibling, prev-sibling",
				other
			),
		}
	}
}

impl core::fmt::Display for NavigateTo {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Parent => write!(f, "parent"),
			Self::FirstChild => write!(f, "first-child"),
			Self::NextSibling => write!(f, "next-sibling"),
			Self::PrevSibling => write!(f, "prev-sibling"),
		}
	}
}


/// Middleware that intercepts `--navigate` and resolves the target
/// route from the [`RouteTree`]. If the param is absent, calls the
/// inner handler via [`Next`].
#[action]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn NavigateHandler(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (mut request, next) = cx.take();

	let Some(value) = request.get_param("navigate").map(|val| val.to_string())
	else {
		return next.call(request).await;
	};

	let direction = NavigateTo::from_str_param(&value)?;
	let current_path = request.path().clone();
	let action_entity = caller.id();
	let world = caller.world();

	let resolved = world
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(move |query| {
			let tree = query.get(action_entity)?;
			let target_path =
				resolve_navigation(tree, &current_path, direction)?;
			tree.find(&target_path).cloned().xok()
		})
		.await?;

	let Some(node) = resolved else {
		return Response::from_status_body(
			StatusCode::NOT_FOUND,
			"Navigation target not found",
			MediaType::Text,
		)
		.xok();
	};

	// Dispatch through the route's ExchangeAction
	node.merge_path_params(&mut request);
	let entity = world.entity(node.entity);
	let exchange = entity.clone().get_cloned::<ExchangeAction>().await?;
	let response = exchange.call(entity, request).await?;

	response.xok()
}

/// Resolve a navigation direction against a [`RouteTree`] from the
/// given current path, returning the target path segments.
fn resolve_navigation(
	tree: &RouteTree,
	current_path: &[SmolStr],
	direction: NavigateTo,
) -> Result<Vec<SmolStr>> {
	match direction {
		NavigateTo::Parent => resolve_parent(current_path),
		NavigateTo::FirstChild => resolve_first_child(tree, current_path),
		NavigateTo::NextSibling => {
			resolve_sibling(tree, current_path, SiblingDirection::Next)
		}
		NavigateTo::PrevSibling => {
			resolve_sibling(tree, current_path, SiblingDirection::Prev)
		}
	}
}

/// Navigate to the parent by dropping the last path segment.
/// An empty path stays at root.
fn resolve_parent(current_path: &[SmolStr]) -> Result<Vec<SmolStr>> {
	if current_path.is_empty() {
		vec![].xok()
	} else {
		current_path[..current_path.len() - 1].to_vec().xok()
	}
}

/// Navigate to the first child of the current path's subtree.
fn resolve_first_child(
	tree: &RouteTree,
	current_path: &[SmolStr],
) -> Result<Vec<SmolStr>> {
	let subtree = if current_path.is_empty() {
		tree
	} else {
		tree.find_subtree(current_path).ok_or_else(|| {
			bevyhow!("No route found at /{}", current_path.join("/"))
		})?
	};

	let first = find_first_node_child(subtree).ok_or_else(|| {
		bevyhow!("No child routes under /{}", current_path.join("/"))
	})?;

	path_segments(&first.path)
}

/// Recursively find the first child subtree with a node.
fn find_first_node_child(tree: &RouteTree) -> Option<&RouteTree> {
	for child in &tree.children {
		if child.node().is_some() {
			return Some(child);
		}
		if let Some(found) = find_first_node_child(child) {
			return Some(found);
		}
	}
	None
}

enum SiblingDirection {
	Next,
	Prev,
}

/// Navigate to the next or previous sibling at the same tree level, wrapping at
/// the ends (the history-style shortcut behavior). For non-wrapping slide
/// navigation see [`resolve_slide`].
fn resolve_sibling(
	tree: &RouteTree,
	current_path: &[SmolStr],
	direction: SiblingDirection,
) -> Result<Vec<SmolStr>> {
	let (siblings, current_idx) = siblings_and_index(tree, current_path)?;
	let target_idx = match direction {
		SiblingDirection::Next => (current_idx + 1) % siblings.len(),
		SiblingDirection::Prev => {
			if current_idx == 0 {
				siblings.len() - 1
			} else {
				current_idx - 1
			}
		}
	};
	path_segments(&siblings[target_idx].path)
}

/// The routable siblings at the current path's tree level, paired with the
/// current path's index among them. Shared by [`resolve_sibling`] (wrapping
/// history shortcuts) and [`resolve_slide`] (clamped slide navigation).
fn siblings_and_index<'a>(
	tree: &'a RouteTree,
	current_path: &[SmolStr],
) -> Result<(Vec<&'a RouteTree>, usize)> {
	if current_path.is_empty() {
		bevybail!("Cannot navigate to a sibling of the root");
	}

	let parent_path = &current_path[..current_path.len() - 1];
	let current_segment = &current_path[current_path.len() - 1];

	let parent_subtree = if parent_path.is_empty() {
		tree
	} else {
		tree.find_subtree(parent_path).ok_or_else(|| {
			bevyhow!("No route found at /{}", parent_path.join("/"))
		})?
	};

	let siblings: Vec<&RouteTree> = parent_subtree
		.children
		.iter()
		.filter(|child| child.node().is_some())
		.collect();

	if siblings.is_empty() {
		bevybail!("No sibling routes at this level");
	}

	let current_idx = siblings
		.iter()
		.position(|child| {
			child
				.path
				.last()
				.map(|seg| seg.name() == current_segment)
				.unwrap_or(false)
		})
		.ok_or_else(|| {
			bevyhow!(
				"Current path /{} not found among siblings",
				current_path.join("/")
			)
		})?;

	(siblings, current_idx).xok()
}

/// Marker opting a router into keyboard slide navigation (the arrow/space/page
/// keys step between sibling routes). `beet present` inserts it on the deck
/// router; ordinary `serve`/docs-TUI routers omit it, so their plain arrows keep
/// scrolling the page. Read by the `slide_nav` system in [`NavigatorPlugin`].
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Default, Component)]
pub struct SlideDeck;

/// A slide-deck navigation step, resolved against the flat list of sibling
/// routes at the current path's level. Unlike [`NavigateTo`]'s sibling moves
/// these clamp at the ends rather than wrapping: a deck does not loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum SlideNav {
	/// The previous sibling, clamped at the first.
	Prev,
	/// The next sibling, clamped at the last.
	Next,
	/// The first sibling.
	First,
	/// The last sibling.
	Last,
}

/// The path of the `n`th (1-based) top-level slide, clamped to the deck range
/// (`n < 1` lands on the first, `n > len` on the last). A deck is a flat list,
/// so its slides are the routable children at the tree root, in sorted order.
/// Backs `beet present --slide=N`.
pub fn resolve_nth_slide(tree: &RouteTree, n: usize) -> Result<Vec<SmolStr>> {
	let slides: Vec<&RouteTree> = tree
		.children
		.iter()
		.filter(|child| child.node().is_some())
		.collect();
	if slides.is_empty() {
		bevybail!("deck has no slides");
	}
	let idx = n.saturating_sub(1).min(slides.len() - 1);
	path_segments(&slides[idx].path)
}

/// Resolve a [`SlideNav`] step against a [`RouteTree`] from the current path,
/// returning the target path segments. Clamps at the ends (no wrap).
pub fn resolve_slide(
	tree: &RouteTree,
	current_path: &[SmolStr],
	nav: SlideNav,
) -> Result<Vec<SmolStr>> {
	let (siblings, current_idx) = siblings_and_index(tree, current_path)?;
	let target_idx = match nav {
		SlideNav::Prev => current_idx.saturating_sub(1),
		SlideNav::Next => (current_idx + 1).min(siblings.len() - 1),
		SlideNav::First => 0,
		SlideNav::Last => siblings.len() - 1,
	};
	path_segments(&siblings[target_idx].path)
}

/// Extract the segment names from a [`PathPattern`] as a `Vec<SmolStr>`.
fn path_segments(pattern: &PathPattern) -> Result<Vec<SmolStr>> {
	pattern
		.iter()
		.map(|seg| seg.name().into())
		.collect::<Vec<_>>()
		.xok()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A minimal router bundle for the navigation tests: the [`Router`] dispatch
	/// action plus the [`NavigateHandler`] middleware under test, and nothing
	/// else. Unlike [`default_router`], it wires no opinionated app routes, so
	/// the sibling/first-child ordering these tests assert is not perturbed.
	fn nav_router() -> impl Bundle { (Router, NavigateHandler::default()) }

	#[beet_core::test]
	fn navigate_to_from_str() {
		NavigateTo::from_str_param("parent")
			.unwrap()
			.xpect_eq(NavigateTo::Parent);
		NavigateTo::from_str_param("first-child")
			.unwrap()
			.xpect_eq(NavigateTo::FirstChild);
		NavigateTo::from_str_param("next-sibling")
			.unwrap()
			.xpect_eq(NavigateTo::NextSibling);
		NavigateTo::from_str_param("prev-sibling")
			.unwrap()
			.xpect_eq(NavigateTo::PrevSibling);
		NavigateTo::from_str_param("bogus").unwrap_err();
	}

	#[beet_core::test]
	fn navigate_to_display() {
		NavigateTo::Parent.to_string().xpect_eq("parent");
		NavigateTo::FirstChild.to_string().xpect_eq("first-child");
		NavigateTo::NextSibling.to_string().xpect_eq("next-sibling");
		NavigateTo::PrevSibling.to_string().xpect_eq("prev-sibling");
	}

	#[beet_core::test]
	async fn navigate_parent_from_child() {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				render_action::fixed_func_route("", || rsx! { <h1>"Root"</h1> }),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"About page"</p> }
				),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str(
				"about --navigate=parent",
			))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Root").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_first_child() {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				render_action::fixed_func_route(
					"alpha",
					|| rsx! { <p>"Alpha page"</p> }
				),
				render_action::fixed_func_route(
					"beta",
					|| rsx! { <p>"Beta page"</p> }
				),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str(
				"--navigate=first-child",
			))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Alpha page").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_next_sibling_wraps() {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				render_action::fixed_func_route(
					"alpha",
					|| rsx! { <p>"Alpha page"</p> }
				),
				render_action::fixed_func_route(
					"beta",
					|| rsx! { <p>"Beta page"</p> }
				),
			]))
			.flush();

		// alpha -> next -> beta
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str(
				"alpha --navigate=next-sibling",
			))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Beta page").xpect_true();

		// beta -> next -> wraps to alpha
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str(
				"beta --navigate=next-sibling",
			))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Alpha page").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_prev_sibling_wraps() {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				render_action::fixed_func_route(
					"alpha",
					|| rsx! { <p>"Alpha page"</p> }
				),
				render_action::fixed_func_route(
					"beta",
					|| rsx! { <p>"Beta page"</p> }
				),
			]))
			.flush();

		// alpha -> prev -> wraps to beta
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::from_cli_str(
				"alpha --navigate=prev-sibling",
			))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Beta page").xpect_true();
	}

	/// A flat three-route deck, the [`RouteTree`] slide-nav resolves against.
	fn slide_deck() -> RouteTree {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				render_action::fixed_route("alpha", rsx! { <p>"a"</p> }),
				render_action::fixed_route("beta", rsx! { <p>"b"</p> }),
				render_action::fixed_route("gamma", rsx! { <p>"c"</p> }),
			]))
			.flush();
		world.entity(root).get::<RouteTree>().unwrap().clone()
	}

	/// The single path segment a [`SlideNav`] step lands on from `current`.
	fn slide_to(tree: &RouteTree, current: &str, nav: SlideNav) -> String {
		resolve_slide(tree, &[current.into()], nav).unwrap().join("/")
	}

	/// Next/prev clamp at the ends rather than wrapping (a deck does not loop).
	#[beet_core::test]
	fn slide_next_prev_clamp_at_ends() {
		let tree = slide_deck();
		// interior moves
		slide_to(&tree, "alpha", SlideNav::Next).xpect_eq("beta");
		slide_to(&tree, "beta", SlideNav::Next).xpect_eq("gamma");
		slide_to(&tree, "gamma", SlideNav::Prev).xpect_eq("beta");
		// next on the last slide stays put; prev on the first stays put
		slide_to(&tree, "gamma", SlideNav::Next).xpect_eq("gamma");
		slide_to(&tree, "alpha", SlideNav::Prev).xpect_eq("alpha");
	}

	/// First/last jump to the ends regardless of the current slide.
	#[beet_core::test]
	fn slide_first_and_last() {
		let tree = slide_deck();
		slide_to(&tree, "beta", SlideNav::First).xpect_eq("alpha");
		slide_to(&tree, "beta", SlideNav::Last).xpect_eq("gamma");
		slide_to(&tree, "alpha", SlideNav::First).xpect_eq("alpha");
		slide_to(&tree, "gamma", SlideNav::Last).xpect_eq("gamma");
	}

	/// `--slide=N` maps a 1-based index to the Nth ordered slide, clamping an
	/// out-of-range N to the first/last rather than erroring.
	#[beet_core::test]
	fn nth_slide_maps_and_clamps() {
		let tree = slide_deck();
		let nth = |n| resolve_nth_slide(&tree, n).unwrap().join("/");
		// 1-based: slide 1 is the first, 3 the last
		nth(1).xpect_eq("alpha");
		nth(2).xpect_eq("beta");
		nth(3).xpect_eq("gamma");
		// out of range clamps to the ends
		nth(0).xpect_eq("alpha");
		nth(99).xpect_eq("gamma");
	}

	#[beet_core::test]
	async fn navigate_without_param_passes_through() {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![render_action::fixed_func_route(
				"about",
				|| rsx! { <p>"About page"</p> }
			),]))
			.flush();

		// No --navigate param, should route normally
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("About page").xpect_true();
	}
}
