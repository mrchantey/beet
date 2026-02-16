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

/// Stock counter page using MDX-style markdown interpolation for
/// mixed static content and interactive tools.
fn counter() -> impl Bundle {
	let field_ref = FieldRef::new("count").init_with(0);

	(
		card("counter"),
		mdx!(
			r#"
# Stock Counter
Records in stock: { field_ref.clone().as_text() }

## Tools
{ increment(field_ref) }
"#
		),
	)
}
