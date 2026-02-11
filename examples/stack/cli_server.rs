//! # CLI Server
//!
//! Demonstrates a single-shot CLI application using [`cli_server`].
//! The app parses process arguments, dispatches them through a
//! [`markdown_interface`], and prints the response to stdout.
//!
//! ## Running the Example
//!
//! ```sh
//! # show help
//! cargo run --example cli_server --features stack -- --help
//!
//! # call the increment tool
//! cargo run --example cli_server --features stack -- increment
//!
//! # navigate to a card
//! cargo run --example cli_server --features stack -- about
//! ```
use beet::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
	app.world_mut().spawn((
		Card,
		markdown_interface(),
		cli_server(),
		children![about(), counter(),],
	));
	async_ext::block_on(app.run_async());
}


fn about() -> impl Bundle {
	(card("about"), Title::with_text("About"), children![
		Paragraph::with_text("howdy doody!")
	])
}


fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count");

	(
		card("counter"),
		Title::with_text("Counter"),
		increment(field_ref.clone()),
		children![Paragraph::with_text("The count is "), field_ref.as_text()],
	)
}
