//! Directional navigation within a [`RouteTree`].
//!
//! The [`navigate_handler`] checks for a `--navigate` param and
//! resolves the target path relative to the current request path,
//! then calls the target tool directly for rendering.

use crate::prelude::*;
use beet_core::prelude::*;

/// The direction to navigate relative to the current path.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum NavigateTo {
	/// Move to the parent card or root.
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

impl std::fmt::Display for NavigateTo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Parent => write!(f, "parent"),
			Self::FirstChild => write!(f, "first-child"),
			Self::NextSibling => write!(f, "next-sibling"),
			Self::PrevSibling => write!(f, "prev-sibling"),
		}
	}
}


/// Checks for the `--navigate` param and resolves the target route
/// directly from the [`RouteTree`].
///
/// All routes are tools now, so navigation simply calls the target
/// tool with the request. The current position is taken from the
/// request path segments.
pub(crate) async fn navigate_handler(
	cx: AsyncToolContext<Request>,
) -> Result<Outcome<Response, Request>> {
	let Some(value) = cx.get_param("navigate") else {
		return Fail(cx.input).xok();
	};

	let direction = NavigateTo::from_str_param(value)?;
	let current_path = cx.input.path().clone();
	let tool_entity = cx.tool.id();
	let world = cx.tool.world();

	let resolved = world
		.with_then(move |world: &mut World| -> Result<Option<ToolNode>> {
			let tree = root_route_tree(world, tool_entity)?;
			let target_path =
				resolve_navigation(tree, &current_path, direction)?;
			tree.find(&target_path).cloned().xok()
		})
		.await?;

	let Some(node) = resolved else {
		return Pass(Response::ok_body(
			"Navigation target not found",
			"text/plain",
		))
		.xok();
	};

	// All routes are tools, call them directly
	let response = world
		.entity(node.entity)
		.call::<Request, Response>(cx.input)
		.await?;

	Pass(response).xok()
}

/// Resolve a navigation direction against a [`RouteTree`] from the
/// given current path, returning the target path segments.
fn resolve_navigation(
	tree: &RouteTree,
	current_path: &[String],
	direction: NavigateTo,
) -> Result<Vec<String>> {
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
fn resolve_parent(current_path: &[String]) -> Result<Vec<String>> {
	if current_path.is_empty() {
		vec![].xok()
	} else {
		current_path[..current_path.len() - 1].to_vec().xok()
	}
}

/// Navigate to the first child of the current path's subtree.
fn resolve_first_child(
	tree: &RouteTree,
	current_path: &[String],
) -> Result<Vec<String>> {
	let subtree = if current_path.is_empty() {
		tree
	} else {
		tree.find_subtree(current_path).ok_or_else(|| {
			bevyhow!("No route found at /{}", current_path.join("/"))
		})?
	};

	// Find the first child that has a node
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
		// intermediate nodes without a route may have children
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

/// Navigate to the next or previous sibling at the same tree level.
fn resolve_sibling(
	tree: &RouteTree,
	current_path: &[String],
	direction: SiblingDirection,
) -> Result<Vec<String>> {
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

	// Collect children that have nodes (visible routes)
	let siblings: Vec<&RouteTree> = parent_subtree
		.children
		.iter()
		.filter(|child| child.node().is_some())
		.collect();

	if siblings.is_empty() {
		bevybail!("No sibling routes at this level");
	}

	// Find current index by matching the last segment name
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

/// Extract the segment names from a [`PathPattern`] as a `Vec<String>`.
fn path_segments(pattern: &PathPattern) -> Result<Vec<String>> {
	pattern
		.iter()
		.map(|seg| seg.name().to_string())
		.collect::<Vec<_>>()
		.xok()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
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

	#[test]
	fn navigate_to_display() {
		NavigateTo::Parent.to_string().xpect_eq("parent");
		NavigateTo::FirstChild.to_string().xpect_eq("first-child");
		NavigateTo::NextSibling.to_string().xpect_eq("next-sibling");
		NavigateTo::PrevSibling.to_string().xpect_eq("prev-sibling");
	}

	#[beet_core::test]
	async fn navigate_parent_from_child() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_interface(), children![
				card("", || Heading1::with_text("Root")),
				(card("about", || Paragraph::with_text("About page")),),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("about --navigate=parent").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		// Navigating parent from /about should render root card
		body.contains("Root").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_first_child() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_interface(), children![
				card("alpha", || Paragraph::with_text("Alpha page")),
				card("beta", || Paragraph::with_text("Beta page")),
			]))
			.flush();

		let body = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("--navigate=first-child").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		// First child should be alpha (sorted alphabetically)
		body.contains("Alpha page").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_next_sibling_wraps() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_interface(), children![
				card("alpha", || Paragraph::with_text("Alpha page")),
				card("beta", || Paragraph::with_text("Beta page")),
			]))
			.flush();

		// alpha -> next -> beta
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("alpha --navigate=next-sibling").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Beta page").xpect_true();

		// beta -> next -> wraps to alpha
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("beta --navigate=next-sibling").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Alpha page").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_prev_sibling_wraps() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_interface(), children![
				card("alpha", || Paragraph::with_text("Alpha page")),
				card("beta", || Paragraph::with_text("Beta page")),
			]))
			.flush();

		// alpha -> prev -> wraps to beta
		let body = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("alpha --navigate=prev-sibling").unwrap(),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("Beta page").xpect_true();
	}

	#[beet_core::test]
	async fn navigate_without_param_passes_through() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_interface(), children![card("about", || {
				Paragraph::with_text("About page")
			}),]))
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
