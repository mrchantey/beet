use anyhow::Result;
use pulldown_cmark::CowStr;
use pulldown_cmark::Event;
use pulldown_cmark::MetadataBlockKind;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;
use syn::Block;


/// Collection of functions for parsing markdown
pub struct ParseMarkdown;

impl ParseMarkdown {
	fn options() -> Options {
		Options::ENABLE_TABLES
				| Options::ENABLE_FOOTNOTES
				| Options::ENABLE_STRIKETHROUGH
				| Options::ENABLE_TASKLISTS
				// replaces ' with ’ etc, if users want this they should do a find and 
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

	/// Parse a given string of markdown into an rsx string,
	/// often parsed by [`StringToWebTokens`]
	pub fn markdown_to_rsx_str(markdown: &str) -> String {
		let parser = Parser::new_ext(&markdown, Self::options());

		let approx_out_len = markdown.len() * 3 / 2;
		let mut html_output = String::with_capacity(approx_out_len);
		pulldown_cmark::html::push_html(&mut html_output, parser);
		html_output
			.trim() // pulldown-cmark inserts a trailing \n
			.to_string()
	}

	/// returns the content of the first frontmatter block discovered,
	/// wrapped in parentheses as a requirement of the `ron` parser
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
				anyhow::bail!(
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


	/// a custom parser for the frontmatter
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
						anyhow::anyhow!("frontmatter line has no key: {line}")
					})?
					.trim();
				let value = split
					.next()
					.ok_or_else(|| {
						anyhow::anyhow!("frontmatter line has no value: {line}")
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
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

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
		expect(ParseMarkdown::markdown_to_rsx_str(MARKDOWN))
			.to_be("<h1>hello world</h1>");
	}

	#[test]
	#[ignore = "todo"]
	// currently text nodes of html tags are not parsed
	fn nested_markdown() {
		expect(ParseMarkdown::markdown_to_rsx_str(
			r#"<div>## Subheading</div>"#,
		))
		.to_be("<div><h2>Subheading</h2></div>\n");
	}

	#[test]
	fn frontmatter() {
		let frontmatter =
			ParseMarkdown::extract_frontmatter_string(MARKDOWN).unwrap();
		let frontmatter: Frontmatter = toml::from_str(&frontmatter.0).unwrap();
		expect(frontmatter).to_be(Frontmatter {
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
	#[ignore = "todo"]
	fn yaml_frontmatter() {
		// let yaml = r#"
		// let foo = DocsMeta {
		// 	title: Some("Beet Site".into()),
		// 	description: Some("A very bevy metaframework".into()),
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
