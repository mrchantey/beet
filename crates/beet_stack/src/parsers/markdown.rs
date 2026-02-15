use beet_core::prelude::*;
use pulldown_cmark::CowStr;
use pulldown_cmark::Event;
use pulldown_cmark::MetadataBlockKind;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;
use pulldown_cmark::TextMergeStream;






pub struct MarkdownParser {
	options: Options,
}

impl Default for MarkdownParser {
	fn default() -> Self {
		Self {
			options: Self::default_options(),
		}
	}
}

impl MarkdownParser {
	fn new() -> Self { Self::default() }

	/// Returns the pulldown-cmark options used for parsing.
	fn default_options() -> Options {
		Options::ENABLE_TABLES
				| Options::ENABLE_FOOTNOTES
				| Options::ENABLE_STRIKETHROUGH
				| Options::ENABLE_TASKLISTS
				// replaces ' with ' etc, if users want this they should do a find and
				// replace at a higher level
				// | Options::ENABLE_SMART_PUNCTUATION
				| Options::ENABLE_HEADING_ATTRIBUTES
				| Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
				| Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
				// | Options::ENABLE_OLD_FOOTNOTES
				| Options::ENABLE_MATH
				| Options::ENABLE_GFM
				| Options::ENABLE_DEFINITION_LIST
				| Options::ENABLE_SUPERSCRIPT
				| Options::ENABLE_SUBSCRIPT
				| Options::ENABLE_WIKILINKS
	}

	pub fn diff(&self, entity: EntityWorldMut, text: &str) {
		let parser = Parser::new_ext(text, self.options);
		// collapse adjacent text
		let parser = TextMergeStream::new(parser);



		for event in parser {
			match event {
				Event::Start(Tag::Paragraph) => todo!(),
				Event::Start(tag) => todo!(),
				Event::End(tag_end) => todo!(),
				Event::Text(cow_str) => todo!(),
				Event::Code(cow_str) => todo!(),
				Event::InlineMath(cow_str) => todo!(),
				Event::DisplayMath(cow_str) => todo!(),
				Event::Html(cow_str) => todo!(),
				Event::InlineHtml(cow_str) => todo!(),
				Event::FootnoteReference(cow_str) => todo!(),
				Event::SoftBreak => todo!(),
				Event::HardBreak => todo!(),
				Event::Rule => todo!(),
				Event::TaskListMarker(_) => todo!(),
			}
		}
	}

	fn diff_inner<'a>(
		&self,
		entity: EntityWorldMut,
		parser: impl Iterator<Item = Event<'a>>,
	) {
		// nested parse calls?
	}
}
