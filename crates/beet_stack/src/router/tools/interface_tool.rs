//! An interface routes requests to cards and tools.
//!
//! This module provides [`default_interface`], an async interface that
//! handles request routing, card navigation, tool invocation, and help
//! rendering. It delegates to shared functions in [`render_markdown`]
//! and [`help`] rather than duplicating their logic.
//!
//! ## Routing Behavior
//!
//! - **Cards**: tool-based routes created via [`card`] that delegate
//!   rendering to the nearest [`RenderToolMarker`] entity.
//! - **`--help`**: scoped to the requested path prefix, ie
//!   `counter --help` only shows routes under `/counter`.
//! - **Not found**: shows help scoped to the nearest ancestor card,
//!   ie `counter nonsense` shows help for `/counter`.

use crate::prelude::*;
use beet_core::prelude::*;

/// Create an interface from a handler, inserting an [`Interface`]
/// component on the entity.
pub fn interface() -> impl Bundle { (RouteHidden, exchange_fallback()) }
/// A Request/Response tool that will try each children until an
/// Outcome::Response is reached, or else returns a NotFound.
/// Errors are converted to a response.
pub fn exchange_fallback() -> impl Bundle {
	(
		// Name::new("Exchange Fallback"),
		tool(async |request: AsyncToolContext<Request>| -> Response {
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
				// if the returned error is a HttpError, its status code will be used.
				Err(err) => HttpError::from_opaque(err).into_response(),
			}
		}),
	)
}

/// Creates a standard markdown interface with help, navigation,
/// routing, and fallback handlers as a child fallback chain.
///
/// Includes a [`markdown_render_tool`] so that cards can render
/// their content to markdown by default.
///
/// The handler chain runs in order:
/// 1. **Help** — if `--help` is present, render help scoped to the
///    request path prefix.
/// 2. **Navigate** — if `--navigate` is present, resolve the
///    navigation direction relative to the current path.
/// 3. **Router** — look up the path in the [`RouteTree`]. All routes
///    are tools; cards delegate to the render tool internally.
/// 4. **Contextual Not Found** — show help for the nearest ancestor
///    card of the unmatched path.
pub fn default_router() -> impl Bundle {
	(
		interface(),
		OnSpawn::insert(children![
			markdown_render_tool(),
			(Name::new("Help Tool"), RouteHidden, tool(help_handler)),
			(
				Name::new("Navigate Tool"),
				RouteHidden,
				tool(navigate_handler)
			),
			try_router(),
			(
				Name::new("Contextual Not Found"),
				RouteHidden,
				tool(contextual_not_found_handler)
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
			tool(
				|req: In<ToolContext<Request>>,
				 trees: Query<&RouteTree>|
				 -> Result<RouteTree> {
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
		tree.find(&["add"]).xpect_some();
		tree.find(&["add"])
			.unwrap()
			.path
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}

	#[beet_core::test]
	async fn dispatches_tool_request() {
		StackPlugin::world()
			.spawn((default_router(), children![route_tool(
				"add",
				|(a, b): (i32, i32)| -> i32 { a + b }
			)]))
			.call::<Request, Response>(
				Request::with_json("add", &(1i32, 2i32)).unwrap(),
			)
			.await
			.unwrap()
			.json::<i32>()
			.await
			.unwrap()
			.xpect_eq(3);
	}

	#[beet_core::test]
	async fn help_flag_returns_route_list() {
		StackPlugin::world()
			.spawn((default_router(), children![
				increment(FieldRef::new("count")),
				card("about", || Paragraph::with_text("about")),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn dispatches_help_request() {
		StackPlugin::world()
			.spawn((default_router(), children![
				increment(FieldRef::new("count")),
				card("about", || Paragraph::with_text("about")),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn not_found() {
		StackPlugin::world()
			.spawn((default_router(), children![increment(FieldRef::new(
				"count"
			))]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NotFound);
	}

	#[beet_core::test]
	async fn renders_root_card_on_empty_args() {
		StackPlugin::world()
			.spawn((default_router(), children![
				card("", || {
					children![
						Heading1::with_text("My Server"),
						Paragraph::with_text("welcome!"),
					]
				}),
				card("about", || Paragraph::with_text("about")),
			]))
			.call::<Request, Response>(Request::from_cli_str("").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("My Server")
			.xpect_contains("welcome!");
	}

	#[beet_core::test]
	async fn scoped_help_for_subcommand() {
		let mut world = StackPlugin::world();

		let root = world
			.spawn((default_router(), children![
				(
					card("counter", || Paragraph::with_text("counter")),
					children![increment(FieldRef::new("count")),],
				),
				card("about", || Paragraph::with_text("about")),
			]))
			.flush();

		let res = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("counter --help").unwrap(),
			)
			.await
			.unwrap();

		let body = res.unwrap_str().await;
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}
}
