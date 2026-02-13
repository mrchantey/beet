//! An interface routes requests to cards and tools.
//!
//! This module provides [`default_interface`], an async interface that
//! handles request routing, card navigation, tool invocation, and help
//! rendering. It delegates to shared functions in [`render_markdown`]
//! and [`help`] rather than duplicating their logic.
//!
//! ## Routing Behavior
//!
//! - **Root card**: a [`Card`] with no [`PathPartial`] matches the
//!   empty path, serving as the root content.
//! - **`--help`**: scoped to the requested path prefix, ie
//!   `counter --help` only shows routes under `/counter`.
//! - **Not found**: shows help scoped to the nearest ancestor card,
//!   ie `counter nonsense` shows help for `/counter`.

use crate::prelude::*;
use beet_core::prelude::*;

/// An interface for interacting with a card-based application.
///
/// Interfaces provide a way to navigate between cards and call tools
/// within the current card context. The [`Interface`] component tracks
/// the currently active card, enabling REPL-like navigation:
///
/// ```text
/// > my_app
/// // prints help for root: subcommands: foo
/// > foo
/// // prints foo in markdown, and sets current card to foo
/// > --help
/// // prints help for foo, not the root: subcommands: bar
/// > bar
/// // goes to route foo/bar, if bar is a tool dont update current card.
/// ```
///
/// Help is handled by the interface itself, not added to each route.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Interface {}

impl Interface {}

/// Create an interface from a handler, inserting an [`Interface`]
/// pointing to itself as the current card.
pub fn interface() -> impl Bundle {
	(
		Interface::default(),
		ExchangeToolMarker,
		RouteHidden,
		direct_tool(async |request: AsyncToolContext<Request>| -> Response {
			match fallback::<Request, Response>(request).await {
				// a response matched, which may be an opinionated not found response
				Ok(Pass(res)) => res,
				// usually an interface should render an opinionated not found response
				// as the final fallback, in this case they didnt so we'll return
				// a simple plaintext one.
				Ok(Fail(req)) => Response::from_status_body(
					StatusCode::NotFound,
					format!("Resource not found: {}", req.path_string()),
					"text/plain",
				),
				// if the returned error is a HttpError its status code will be used.
				Err(err) => HttpError::from_opaque(err).into_response(),
			}
		}),
	)
}

/// Creates a standard markdown interface with help, navigation,
/// routing, and fallback handlers as a child fallback chain.
///
/// The handler chain runs in order:
/// 1. **Help** — if `--help` is present, render help scoped to the
///    request path prefix.
/// 2. **Navigate** — if `--navigate` is present, resolve the
///    navigation direction relative to the current path.
/// 3. **Router** — look up the path in the [`RouteTree`]. Cards are
///    rendered as markdown, tools are called directly. A [`Card`] with
///    no [`PathPartial`] naturally matches the empty path.
/// 4. **Contextual Not Found** — show help for the nearest ancestor
///    card of the unmatched path.
pub fn default_interface() -> impl Bundle {
	(
		interface(),
		OnSpawn::insert(children![
			(
				Name::new("Help Tool"),
				RouteHidden,
				direct_tool(help_handler)
			),
			(
				Name::new("Navigate Tool"),
				RouteHidden,
				direct_tool(navigate_handler)
			),
			(Name::new("Router"), RouteHidden, direct_tool(route_handler)),
			(
				Name::new("Contextual Not Found"),
				RouteHidden,
				direct_tool(contextual_not_found_handler)
			)
		]),
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn my_interface() -> impl Bundle {
		(
			Interface::default(),
			direct_tool(
				|req: In<ToolContext<Request>>,
				 trees: Query<&RouteTree>,
				 interfaces: Query<&Interface>|
				 -> Result<RouteTree> {
					let _interface = interfaces.get(req.tool)?;
					let tree = trees.get(req.tool)?;
					Ok(tree.clone())
				},
			),
			children![(
				PathPartial::new("add"),
				tool(|(a, b): (u32, u32)| a + b)
			)],
		)
	}


	#[test]
	fn works() {
		let tree = RouterPlugin::world()
			.spawn(my_interface())
			.call_blocking::<_, RouteTree>(Request::get("foo"))
			.unwrap();
		tree.find_tool(&["add"]).xpect_some();
		tree.find_tool(&["add"])
			.unwrap()
			.path
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}
}
