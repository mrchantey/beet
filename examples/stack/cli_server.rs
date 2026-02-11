//! # CLI Server
//!
//! Demonstrates a single-shot CLI application using [`cli_server`].
//! The app parses process arguments, dispatches them through a
//! [`markdown_interface`], and prints the response to stdout.
//!
//! The root content is defined by a `card("")` child, which is
//! displayed when no arguments are provided.
//!
//! ## Running the Example
//!
//! ```sh
//! # show root card
//! cargo run --example cli_server --features stack
//!
//! # show help for all routes
//! cargo run --example cli_server --features stack -- --help
//!
//! # navigate to a card
//! cargo run --example cli_server --features stack -- about
//!
//! # show help scoped to a subcommand
//! cargo run --example cli_server --features stack -- counter --help
//!
//! # not-found shows help for nearest ancestor
//! cargo run --example cli_server --features stack -- counter nonsense
//! ```
use beet::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
	app.world_mut()
		.spawn((markdown_interface(), cli_server(), children![
			root(),
			about(),
			counter(),
		]));
	async_ext::block_on(app.run_async());
}


fn root() -> impl Bundle {
	(Card, Title::with_text("My Server"), children![
		Paragraph::with_text("welcome to the server!")
	])
}

fn about() -> impl Bundle {
	(card("about"), Title::with_text("About"), children![
		Paragraph::with_text("howdy doody!")
	])
}


fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count").init_with(0);

	(card("counter"), Title::with_text("Counter"), children![
		increment(field_ref.clone()),
		(Paragraph, children![
			TextContent::new("The count is "),
			field_ref.as_text()
		])
	])
}
