//! TUI rendering via [`TuiRenderer`] and [`CardWalker`].
//!
//! The [`draw_system`] is an exclusive system that walks the card
//! tree via [`CardWalker`] and renders each entity using the
//! visitor-based [`TuiRenderer`].
use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::Frame;

/// Renders the current card's content tree into the terminal each frame.
///
/// Walks the card tree via [`CardWalker`] and renders using
/// [`TuiRenderer`], which implements [`CardVisitor`] for unified
/// rendering of all content types.
pub(super) fn draw_system(world: &mut World) -> Result {
	let card_entity = match world
		.query_filtered::<Entity, With<CurrentCard>>()
		.single(world)
	{
		Ok(entity) => entity,
		Err(_) => {
			// not selected yet, just write 'loading'
			world.resource_mut::<RatatuiContext>().draw(|frame| {
				let text = ratatui::text::Text::from("loading..");
				frame.render_widget(text, frame.area());
			})?;

			return Ok(());
		}
	};

	world.resource_scope(
		|world: &mut World, mut context: Mut<RatatuiContext>| {
			bevy_draw(&mut context, |frame| {
				let area = frame.area();
				let buf = frame.buffer_mut();

				world
					.run_system_once_with(
						tui_render_system,
						(card_entity, area),
					)
					.map(|(inner_buf, span_map)| {
						// Merge the inner buffer into the frame buffer
						let area = inner_buf.area;
						for row in area.y..area.y + area.height {
							for col in area.x..area.x + area.width {
								*buf.cell_mut((col, row)).unwrap() =
									inner_buf[(col, row)].clone();
							}
						}
						// Store the span map for input hit-testing
						world.insert_resource(span_map);
					})
					.map_err(|err| bevyhow!("{err}"))
			})
		},
	)
}

/// [`BevyError`] compatible version of [`RatatuiContext::draw`].
fn bevy_draw(
	cx: &mut RatatuiContext,
	func: impl FnOnce(&mut Frame) -> Result,
) -> Result {
	let mut result = Ok(());
	cx.draw(|frame| {
		result = func(frame);
	})?;
	result
}


#[cfg(test)]
mod test {
	use super::*;
	use ratatui::prelude::Rect;

	/// Render a `CurrentCard` entity into a buffer via `CardWalker` +
	/// `TuiRenderer`, returning the plain text content.
	fn render_current_card(
		world: &mut World,
		width: u16,
		height: u16,
	) -> String {
		let card_entity = world
			.query_filtered::<Entity, With<CurrentCard>>()
			.single(world)
			.unwrap();

		let area = Rect::new(0, 0, width, height);
		let (buf, _span_map) = world
			.run_system_once_with(tui_render_system, (card_entity, area))
			.unwrap();

		buffer_to_text(&buf)
	}

	#[test]
	fn heading_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Heading1, children![TextNode::new(
				"Hello World"
			)])]))
			.id();

		let area = Rect::new(0, 0, 80, 24);
		let (buf, _span_map) = world
			.run_system_once_with(tui_render_system, (entity, area))
			.unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("Hello World");
	}

	#[test]
	fn paragraph_renders_text() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![(Paragraph, children![
				TextNode::new("body text")
			])]))
			.id();

		let area = Rect::new(0, 0, 80, 24);
		let (buf, _span_map) = world
			.run_system_once_with(tui_render_system, (entity, area))
			.unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.xpect_contains("body text");
	}

	#[test]
	fn skips_nested_card() {
		let mut world = World::new();
		let entity = world
			.spawn((CardTool, children![
				(Paragraph, children![TextNode::new("visible")]),
				(CardTool, children![(Paragraph, children![TextNode::new(
					"hidden"
				)])]),
			]))
			.id();

		let area = Rect::new(0, 0, 80, 24);
		let (buf, _tui_area) = world
			.run_system_once_with(tui_render_system, (entity, area))
			.unwrap();

		let rendered: String =
			buf.content().iter().map(|cell| cell.symbol()).collect();
		rendered.as_str().xpect_contains("visible");
		rendered.contains("hidden").xpect_false();
	}

	// -- Integration tests: full pipeline from request to rendered output --

	#[beet_core::test]
	async fn tui_render_tool_produces_renderable_current_card() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				card("home", || {
					children![
						Heading1::with_text("Welcome"),
						Paragraph::with_text("Hello from home"),
					]
				}),
			]))
			.flush();

		// Send a request to the "home" card
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("home"))
			.await
			.unwrap();

		// CurrentCard should now exist
		let current = world
			.query_filtered::<Entity, With<CurrentCard>>()
			.single(&world);
		current.xpect_ok();

		// Render the CurrentCard and verify content
		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("Welcome");
		text.as_str().xpect_contains("Hello from home");
	}

	#[beet_core::test]
	async fn tui_render_root_card_via_empty_request() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				card("", || Paragraph::with_text("Root content")),
			]))
			.flush();

		// Empty path request targets the root card
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap();

		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn tui_render_replaces_current_card_content() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				card("first", || Paragraph::with_text("First page")),
				card("second", || Paragraph::with_text("Second page")),
			]))
			.flush();

		// Render first card
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("first"))
			.await
			.unwrap();
		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("First page");

		// Render second card — should replace the first
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("second"))
			.await
			.unwrap();
		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("Second page");
		text.as_str().xnot().xpect_contains("First page");
	}

	#[beet_core::test]
	async fn tui_render_multi_element_card() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				card("rich", || {
					children![
						Heading1::with_text("Title"),
						Paragraph::with_text("Body paragraph"),
						Heading2::with_text("Subtitle"),
						Paragraph::with_text("More text"),
					]
				}),
			]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("rich"))
			.await
			.unwrap();

		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("Title");
		text.as_str().xpect_contains("Body paragraph");
		text.as_str().xpect_contains("Subtitle");
		text.as_str().xpect_contains("More text");
	}

	/// Proves the root cause of the blank TUI screen: `file_card`
	/// spawns a bare `TextNode` without a wrapping block element
	/// like `Paragraph`. The `TuiRenderer` accumulates spans in
	/// `visit_text` but never flushes them because no block-level
	/// `leave_*` callback fires.
	#[beet_core::test]
	async fn bare_text_node_produces_blank_output() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((router(), children![
				tui_render_tool(),
				// This mimics what file_card does: content is a bare
				// TextNode without a Paragraph wrapper.
				card("raw", || TextNode::new("I should be visible")),
			]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("raw"))
			.await
			.unwrap();

		// CurrentCard exists — the pipeline worked
		world
			.query_filtered::<Entity, With<CurrentCard>>()
			.single(&world)
			.xpect_ok();

		// With the finish() fix, bare TextNode content is now flushed.
		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("I should be visible");
	}

	#[beet_core::test]
	async fn tui_render_default_router_with_card() {
		let mut world = StackPlugin::world();
		let root = world
			.spawn((default_router(), children![
				tui_render_tool(),
				card("about", || { Paragraph::with_text("About this app") }),
			]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap();

		let text = render_current_card(&mut world, 80, 24);
		text.as_str().xpect_contains("About this app");
	}
}
