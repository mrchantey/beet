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
///
/// Two families: the *tree* moves (`parent`/`first-child`/`next-sibling`/
/// `prev-sibling`) walk the [`RouteTree`] generically, wrapping at the ends, and
/// dispatch the target inline; the *card* steps (`next`/`prev`/`first`/`last`)
/// resolve against a [`CardDeck`](crate::prelude::CardDeck)'s flat card list
/// (home-aware, clamping at the ends) and redirect the browser to the resolved
/// card (see [`NavigateHandler`]).
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
	/// Step to the next card in the deck, clamped at the last.
	NextCard,
	/// Step to the previous card in the deck, clamped at the first.
	PrevCard,
	/// Jump to the first card in the deck.
	FirstCard,
	/// Jump to the last card in the deck.
	LastCard,
}

impl NavigateTo {
	/// Parse a CLI/query string into a [`NavigateTo`] variant.
	///
	/// Tree moves: `parent`, `first-child`, `next-sibling`, `prev-sibling`.
	/// Card steps: `next`, `prev`, `first`, `last`.
	pub fn from_str_param(value: &str) -> Result<Self> {
		match value {
			"parent" => Ok(Self::Parent),
			"first-child" => Ok(Self::FirstChild),
			"next-sibling" => Ok(Self::NextSibling),
			"prev-sibling" => Ok(Self::PrevSibling),
			"next" => Ok(Self::NextCard),
			"prev" => Ok(Self::PrevCard),
			"first" => Ok(Self::FirstCard),
			"last" => Ok(Self::LastCard),
			other => bevybail!(
				"Unknown navigate direction '{}'. Expected one of: parent, \
				 first-child, next-sibling, prev-sibling, next, prev, first, last",
				other
			),
		}
	}

	/// Whether this is a card-stack step (resolved against the deck's card list
	/// and redirected) rather than a generic tree move (dispatched inline).
	pub fn is_card(&self) -> bool {
		matches!(
			self,
			Self::NextCard | Self::PrevCard | Self::FirstCard | Self::LastCard
		)
	}
}

impl core::fmt::Display for NavigateTo {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Parent => write!(f, "parent"),
			Self::FirstChild => write!(f, "first-child"),
			Self::NextSibling => write!(f, "next-sibling"),
			Self::PrevSibling => write!(f, "prev-sibling"),
			Self::NextCard => write!(f, "next"),
			Self::PrevCard => write!(f, "prev"),
			Self::FirstCard => write!(f, "first"),
			Self::LastCard => write!(f, "last"),
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

	// resolve the target path against the route tree (card steps resolve against
	// the deck's card list, tree moves against the generic sibling/parent tree).
	let target_path = world
		.with_state::<AncestorQuery<&RouteTree>, Result<Vec<SmolStr>>>(
			move |query| {
				let tree = query.get(action_entity)?;
				resolve_navigation(tree, &current_path, direction)
			},
		)
		.await?;

	// a card step redirects the browser to the resolved card so its address bar
	// updates (the deck's keyboard JS just sets `window.location`) and the last
	// card clamps rather than looping; the generic tree moves keep dispatching
	// inline for programmatic / CLI callers.
	if direction.is_card() {
		let location = format!("/{}", target_path.join("/"));
		return Response::temporary_redirect(location).xok();
	}

	let resolved = world
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(move |query| {
			let tree = query.get(action_entity)?;
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

	// Dispatch through the route's Action<Request, Response> slot
	node.merge_path_params(&mut request);
	let entity = world.entity(node.entity);
	let action = entity.clone().get_cloned::<Action<Request, Response>>().await?;
	let response = entity.call_detached(action, request).await?;

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
		NavigateTo::NextCard => resolve_card(tree, current_path, CardNav::Next),
		NavigateTo::PrevCard => resolve_card(tree, current_path, CardNav::Prev),
		NavigateTo::FirstCard => resolve_card(tree, current_path, CardNav::First),
		NavigateTo::LastCard => resolve_card(tree, current_path, CardNav::Last),
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
/// the ends (the history-style shortcut behavior). For non-wrapping
/// stack-of-cards navigation see [`resolve_card`](crate::prelude::resolve_card).
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
/// current path's index among them. Used by [`resolve_sibling`] (wrapping history
/// shortcuts); the card-stack `resolve_card` resolves against its own
/// page-route-only card list instead.
pub(crate) fn siblings_and_index<'a>(
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

/// Extract the segment names from a [`PathPattern`] as a `Vec<SmolStr>`.
pub(crate) fn path_segments(pattern: &PathPattern) -> Result<Vec<SmolStr>> {
	pattern
		.iter()
		.map(|seg| seg.name().into())
		.collect::<Vec<_>>()
		.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
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
	fn navigate_card_from_str() {
		NavigateTo::from_str_param("next")
			.unwrap()
			.xpect_eq(NavigateTo::NextCard);
		NavigateTo::from_str_param("prev")
			.unwrap()
			.xpect_eq(NavigateTo::PrevCard);
		NavigateTo::from_str_param("first")
			.unwrap()
			.xpect_eq(NavigateTo::FirstCard);
		NavigateTo::from_str_param("last")
			.unwrap()
			.xpect_eq(NavigateTo::LastCard);
		// the card steps redirect; the tree moves dispatch inline.
		NavigateTo::NextCard.is_card().xpect_true();
		NavigateTo::NextSibling.is_card().xpect_false();
	}

	/// A card step over HTTP resolves the deck card and redirects the browser to
	/// it (so its address bar updates), clamping at the last card rather than
	/// wrapping. The deck's keyboard JS relies on this `?navigate=next` redirect.
	#[beet_core::test]
	async fn navigate_card_redirects_and_clamps() {
		let mut world = router_world();
		let root = world
			.spawn((nav_router(), children![
				(
					render_action::fixed_func_route("alpha", || rsx! {
						<p>"a"</p>
					}),
					PageRoute
				),
				(
					render_action::fixed_func_route("beta", || rsx! {
						<p>"b"</p>
					}),
					PageRoute
				),
			]))
			.flush();

		// alpha -> next -> redirect to /beta
		let res = world
			.entity_mut(root)
			.exchange(Request::from_cli_str("alpha --navigate=next"))
			.await;
		res.status().xpect_eq(StatusCode::TEMPORARY_REDIRECT);
		res.parts
			.headers
			.get::<header::Location>()
			.unwrap()
			.unwrap()
			.xpect_eq("/beta");

		// beta -> next -> clamps, redirect back to /beta (no wrap to alpha)
		let res = world
			.entity_mut(root)
			.exchange(Request::from_cli_str("beta --navigate=next"))
			.await;
		res.parts
			.headers
			.get::<header::Location>()
			.unwrap()
			.unwrap()
			.xpect_eq("/beta");
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
				render_action::fixed_func_route(
					"",
					|| rsx! { <h1>"Root"</h1> }
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"About page"</p> }
				),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.exchange(Request::from_cli_str("about --navigate=parent"))
			.await
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
			.exchange(Request::from_cli_str("--navigate=first-child"))
			.await
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
			.exchange(Request::from_cli_str("alpha --navigate=next-sibling"))
			.await
			.unwrap_str()
			.await;
		body.contains("Beta page").xpect_true();

		// beta -> next -> wraps to alpha
		let body = world
			.entity_mut(root)
			.exchange(Request::from_cli_str("beta --navigate=next-sibling"))
			.await
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
			.exchange(Request::from_cli_str("alpha --navigate=prev-sibling"))
			.await
			.unwrap_str()
			.await;
		body.contains("Beta page").xpect_true();
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
			.exchange(Request::get("about"))
			.await
			.unwrap_str()
			.await;
		body.contains("About page").xpect_true();
	}
}
