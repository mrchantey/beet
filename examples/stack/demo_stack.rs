use beet::prelude::*;


/// An interface agnostic stack used to demonstrate
/// the behavior of various servers.
pub fn stack() -> impl Bundle {
	(default_interface(), children![root(), about(), counter()])
}



fn root() -> impl Bundle {
	(Card, children![
		Heading1::with_text("My Stack"),
		(Paragraph, Emphasize, children![
			TextContent::new("Welcome to the "),
			(Important, TextContent::new("coolest")),
			TextContent::new(" stack in the world!"),
		]),
	])
}


fn about() -> impl Bundle {
	(card("about"), children![
		Heading1::with_text("About"),
		Paragraph::with_text(r#"My stack is"#),
	])
}


fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count").init_with(0);

	(card("counter"), children![
		Heading1::with_text("Counter"),
		increment(field_ref.clone()),
		(Paragraph, children![
			TextContent::new("The count is "),
			field_ref.as_text()
		])
	])
}
