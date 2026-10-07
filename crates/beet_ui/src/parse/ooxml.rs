//! Office files as media: a Word file or a slide deck parses through its
//! transcode to HTML, so it renders through every pipeline a page does.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Parses a Word file or a slide deck as the HTML it transcodes to, see
/// [`WordDocument::to_html`] and [`SlideDeck::to_html`], so its headings,
/// lists, tables and the looks a form signals by become elements like any
/// page's.
#[derive(Debug, Default, Clone)]
pub struct OoxmlParser;

impl OoxmlParser {
	/// The media types this parser reads.
	pub const SUPPORTED: [MediaType; 2] = [MediaType::Docx, MediaType::Pptx];

	/// The HTML a Word file or a slide deck transcodes to.
	pub fn to_html(bytes: &MediaBytes) -> Result<String> {
		match bytes.media_type() {
			MediaType::Docx => {
				WordDocument::from_bytes(bytes.bytes())?.to_html()
			}
			MediaType::Pptx => SlideDeck::from_bytes(bytes.bytes())?.to_html(),
			other => bevybail!("`{other}` is not a Word file or a slide deck"),
		}
	}

	/// The [`PageMeta`] a file's core properties declare: its title, its
	/// author and the days it was created and last modified, named as
	/// `component`.
	pub fn declarations(
		bytes: &MediaBytes,
		component: &str,
	) -> Result<RootDeclarations> {
		let core = match bytes.media_type() {
			MediaType::Docx => {
				WordDocument::from_bytes(bytes.bytes())?.core_properties()?
			}
			MediaType::Pptx => {
				SlideDeck::from_bytes(bytes.bytes())?.core_properties()?
			}
			_ => return RootDeclarations::default().xok(),
		};
		let text = |text: &str| DataLiteral::Scalar(Value::Str(text.into()));
		let fields = [
			(!core.title.is_empty()).then(|| ("title", text(&core.title))),
			(!core.author.is_empty()).then(|| {
				("authors", DataLiteral::List(vec![text(&core.author)]))
			}),
			core.created_day()
				.map(|day| ("created", text(&day.to_string()))),
			core.modified_day()
				.map(|day| ("updated", text(&day.to_string()))),
		]
		.into_iter()
		.flatten()
		.map(|(key, value)| (SmolStr::new(key), value))
		.collect::<Vec<_>>();
		match fields.is_empty() {
			true => RootDeclarations::default(),
			false => RootDeclarations(vec![NamedLiteral {
				name: component.into(),
				fields: NamedFields::Struct(fields),
			}]),
		}
		.xok()
	}
}

impl NodeParser for OoxmlParser {
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		if !Self::SUPPORTED.contains(cx.bytes.media_type()) {
			return Err(ParseError::UnsupportedType {
				unsupported: cx.bytes.media_type().clone(),
				supported: Self::SUPPORTED.to_vec(),
			});
		}
		let html = MediaBytes::new_html(Self::to_html(cx.bytes)?);
		BsxParser::html().parse(ParseContext {
			entity: cx.entity,
			bytes: &html,
			path: cx.path,
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A Word file parses to a page and renders back as markdown.
	#[beet_core::test]
	fn renders_a_word_file_as_markdown() {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new(
			MediaType::Docx,
			WordDocument::from_body(
				"<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Plan</w:t></w:r></w:p>\
				 <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc>\
				 <w:tc><w:p><w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>Acme</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
			)
			.unwrap()
			.to_bytes()
			.unwrap(),
		);
		MediaParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(entity, &mut world))
			.unwrap()
			.to_string()
			.xpect_contains(
				"# Plan\n\n<!-- t1 -->\n\n| Name | <mark>Acme</mark> |\n|---|---|\n",
			);
	}
}
