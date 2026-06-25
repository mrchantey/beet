//! Proves the document-binding path: a `.bsx` file whose text holds `@doc:field`
//! bindings is loaded under a [`Document`], and each binding resolves to the
//! document's live value. `bx:scope="user"` prefixes the descendant fields, so
//! `@doc:name` reads `user.name`; `@doc:unread=0` seeds a default when the field
//! is absent. This is the same reactive substrate the website uses.
//!
//! ```sh
//! cargo run --example bsx_bindings --features template
//! ```
use beet::prelude::*;

/// The `.bsx` source, embedded so the example runs from any directory.
const GREETING_BSX: &str = include_str!("../assets/bsx/greeting.bsx");

fn main() {
	let mut world = ui_world();

	// 1. A document holds the state the markup binds to. `unread` is omitted so
	// the `@doc:unread=0` binding seeds its default.
	let doc = world
		.spawn(Document::new(val!({ "user": { "name": "Ada" } })))
		.id();

	// 2. Load the `.bsx` under the document, so its `@doc:field` bindings link
	// to the document before the tree is built.
	let container = {
		let bytes = MediaBytes::new_bsx(GREETING_BSX);
		let mut entity = world.spawn(ChildOf(doc));
		BsxParser::bsx()
			.parse(ParseContext::new(&mut entity, &bytes))
			.unwrap();
		entity.id()
	};
	// 3. Flush the document machinery: bindings sync and the seed writes back.
	world.update_local();
	world.update_local();

	let root = world.entity(container).get::<Children>().unwrap()[0];
	let html = render_html(&mut world, root);

	// 4. The bound values reached the rendered output.
	assert!(
		html.contains("Hello, Ada!"),
		"name binding missing:\n{html}"
	);
	assert!(
		html.contains("You have 0 unread messages."),
		"seeded `unread` binding missing:\n{html}"
	);

	// 5. The seed wrote `user.unread = 0` back into the document.
	let seeded = world
		.entity(doc)
		.get::<Document>()
		.unwrap()
		.get_field::<i64>(&[
			FieldSegment::key("user"),
			FieldSegment::key("unread"),
		])
		.unwrap();
	assert_eq!(
		seeded, 0,
		"expected `user.unread` seeded to 0, got {seeded}"
	);

	cross_log!("rendered `.bsx` with document bindings:\n{html}");
}

/// Render `root` to an HTML string through the substrate's [`HtmlRenderer`].
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}
