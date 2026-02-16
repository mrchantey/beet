use beet::prelude::*;

/// Pete's Beets — a music record store stack driven by markdown files.
///
/// Each card loads its content from a `.md` file via [`FileContent`],
/// which is parsed into a semantic entity tree by [`MarkdownDiffer`].
pub fn stack() -> impl Bundle {
	(default_interface(), children![root(), about(), counter()])
}

fn root() -> impl Bundle {
	(Card, FileContent::new("examples/stack/petes_beets/home.md"))
}

fn about() -> impl Bundle {
	(
		card("about"),
		FileContent::new("examples/stack/petes_beets/about.md"),
	)
}

/// Stock counter page defined programmatically since it uses
/// interactive tools that can't be expressed in markdown.
fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count").init_with(0);

	(card("counter"), children![
		Heading1::with_text("Stock Counter"),
		increment(field_ref.clone()),
		(Paragraph, children![
			TextNode::new("Records in stock: "),
			field_ref.as_text()
		])
	])
}
