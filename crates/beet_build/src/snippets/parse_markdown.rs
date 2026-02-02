//! Markdown parsing utilities for RSX conversion.
//!
//! This module provides functions for converting Markdown content into RSX-compatible
//! HTML strings, and extracting frontmatter metadata from Markdown files.

use beet_core::prelude::*;
use pulldown_cmark::CowStr;
use pulldown_cmark::Event;
use pulldown_cmark::MetadataBlockKind;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;
use syn::Block;


/// Collection of functions for parsing markdown content.
///
/// Provides utilities for:
/// - Converting Markdown to RSX-compatible HTML strings
/// - Extracting and parsing frontmatter metadata
pub(crate) struct ParseMarkdown;

impl ParseMarkdown {
	/// Returns the pulldown-cmark options used for parsing.
	fn options() -> Options {
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

	/// Parses a Markdown string into an RSX-compatible HTML string.
	///
	/// The output is suitable for parsing by the RSX combinator system.
	pub fn markdown_to_rsx_str(markdown: &str) -> String {
		let parser = Parser::new_ext(&markdown, Self::options());

		let approx_out_len = markdown.len() * 3 / 2;
		let mut html_output = String::with_capacity(approx_out_len);
		pulldown_cmark::html::push_html(&mut html_output, parser);
		html_output
			.trim() // pulldown-cmark inserts a trailing \n
			.to_string()
	}

	/// Extracts frontmatter from Markdown and returns it as a syn [`Block`].
	///
	/// Supports TOML frontmatter (delimited by `+++`). YAML frontmatter
	/// (delimited by `---`) is not yet supported.
	///
	/// Returns `Ok(None)` if no frontmatter is present.
	pub fn markdown_to_frontmatter_tokens<'a>(
		markdown: &'a str,
	) -> Result<Option<Block>> {
		let frontmatter = Self::extract_frontmatter_string(markdown);
		// frontmatter
		let tokens = match frontmatter {
			// pluses indicates toml, ie foo = "bar"
			Some((frontmatter, MetadataBlockKind::PlusesStyle)) => {
				let frontmatter = frontmatter.to_string();
				Some(syn::parse_quote!({
					beet::exports::toml::from_str(#frontmatter)
				}))
			}
			// minus indicates yaml, ie foo: "bar"
			Some((_frontmatter, MetadataBlockKind::YamlStyle)) => {
				bevybail!(
					"yaml frontmatter is not yet supported, please use +++ toml +++ frontmatter"
				);
				// let frontmatter = Self::yaml_frontmatter_to_ron(&frontmatter)?;
				// Some(syn::parse_quote!({
				// 	beet::exports::ron::from_str(#frontmatter)
				// }))
			}
			None => None,
		};
		Ok(tokens)
	}

	/// Extracts the raw frontmatter string and its style from Markdown.
	fn extract_frontmatter_string<'a>(
		markdown: &'a str,
	) -> Option<(CowStr<'a>, MetadataBlockKind)> {
		let parser = Parser::new_ext(&markdown, Self::options());

		let mut frontmatter = None;
		let mut in_frontmatter = false;
		let mut meta_block_style = MetadataBlockKind::YamlStyle;
		for ev in parser {
			match ev {
				Event::Start(Tag::MetadataBlock(kind)) => {
					meta_block_style = kind;
					in_frontmatter = true;
				}
				Event::Text(txt) => {
					if in_frontmatter {
						frontmatter = Some(txt);
						break;
					}
				}
				Event::End(TagEnd::MetadataBlock(_)) => {
					in_frontmatter = false;
				}
				_ => {}
			}
		}
		frontmatter.map(|frontmatter| (frontmatter, meta_block_style))
	}


	/// Converts YAML frontmatter to RON format.
	#[allow(unused)]
	fn yaml_frontmatter_to_ron(yaml: &str) -> Result<String> {
		let lines = yaml
			.lines()
			.filter_map(|line| {
				let line = line.trim();
				if line.is_empty() || line.starts_with('#') {
					None
				} else {
					Some(line)
				}
			})
			.map(|line| -> Result<String> {
				let mut split = line.splitn(2, ':');
				let key = split
					.next()
					.ok_or_else(|| {
						bevyhow!("frontmatter line has no key: {line}")
					})?
					.trim();
				let value = split
					.next()
					.ok_or_else(|| {
						bevyhow!("frontmatter line has no value: {line}")
					})?
					.trim();
				Ok(format!("{}: {}", key, value))
			})
			.collect::<Result<Vec<_>, _>>()?
			.join(",\n");
		Ok(format!("({})", lines))
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	const MARKDOWN: &str = r#"
+++
val_bool		=	true
val_int			= 83
val_float		= 3.14
val_string	=	"bar=bazz"
[val_enum]
Bar 				= 42
[val_nested]
val_string	= "foo"
+++
# hello world"#;


	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	enum MyEnum {
		Foo,
		Bar(u32),
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Frontmatter {
		val_bool: bool,
		val_int: u32,
		val_float: Option<f32>,
		val_string: String,
		val_enum: MyEnum,
		val_nested: Nested,
	}

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Nested {
		val_string: String,
	}

	#[test]
	fn html() {
		ParseMarkdown::markdown_to_rsx_str(MARKDOWN)
			.xpect_eq("<h1>hello world</h1>");
	}

	#[test]
	#[ignore = "todo"]
	// currently text nodes of html tags are not parsed
	fn nested_markdown() {
		ParseMarkdown::markdown_to_rsx_str(r#"<div>## Subheading</div>"#)
			.xpect_eq("<div><h2>Subheading</h2></div>\n");
	}

	#[test]
	fn frontmatter() {
		let frontmatter =
			ParseMarkdown::extract_frontmatter_string(MARKDOWN).unwrap();
		let frontmatter: Frontmatter = toml::from_str(&frontmatter.0).unwrap();
		frontmatter.xpect_eq(Frontmatter {
			val_bool: true,
			val_int: 83,
			val_float: Some(3.14),
			val_string: "bar=bazz".into(),
			val_enum: MyEnum::Bar(42),
			val_nested: Nested {
				val_string: "foo".into(),
			},
		});
	}

	#[test]
	fn code_blocks() {
		ParseMarkdown::markdown_to_rsx_str("`let foo = bar;`")
			.xpect_eq("<p><code>let foo = bar;</code></p>");
		ParseMarkdown::markdown_to_rsx_str(
			r#"
```rust
let foo = bar;

let bazz = boo;
```
"#,
		)
		// preserves whitespace
		.xpect_eq("<pre><code class=\"language-rust\">let foo = bar;\n\nlet bazz = boo;\n</code></pre>");
	}

	#[test]
	fn preserves_whitespace() {
		ParseMarkdown::markdown_to_rsx_str("i am **very** cool")
			.xpect_eq("<p>i am <strong>very</strong> cool</p>");
	}

	#[test]
	#[ignore = "todo"]
	fn yaml_frontmatter() {
		// let yaml = r#"
		// let foo = ArticleMeta {
		// 	title: Some("Beet Site".into()),
		// 	description: Some("foo".into()),
		// 	draft: false,
		// 	sidebar: SidebarInfo {
		// 		label: Some("Beet Site".into()),
		// 		..Default::default()
		// 	},
		// };

		// let ron = beet::exports::ron::to_string(&foo).unwrap();
		// println!("Ron: {}", ron);
	}
}
