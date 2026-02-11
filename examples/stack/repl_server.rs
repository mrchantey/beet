//! # REPL Server
//!
//! Demonstrates an interactive REPL application using [`repl_server`].
//! The app reads lines from stdin, dispatches each through a
//! [`markdown_interface`], and prints the response to stdout.
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example repl --features stack
//! ```
//!
//! Then type commands interactively:
//! ```text
//! > --help
//! > increment
//! > about
//! > exit
//! ```
use beet::prelude::*;

fn main() -> AppExit {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
	app.world_mut().spawn((
		Card,
		markdown_interface(),
		repl_server(),
		children![about(), counter()],
	));
	app.run()
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
