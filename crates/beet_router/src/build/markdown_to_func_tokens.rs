use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::HtmlTokensToRust;
use beet_rsx::prelude::StringToHtmlTokens;
use http::Method;
use pulldown_cmark::CowStr;
use pulldown_cmark::Event;
use pulldown_cmark::MetadataBlockKind;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;
use quote::quote;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Block;

pub struct MarkdownToFuncTokens;

// impl MarkdownToFuncTokens

impl MarkdownToFuncTokens {
	fn options() -> Options {
		Options::ENABLE_TABLES
				| Options::ENABLE_FOOTNOTES
				| Options::ENABLE_STRIKETHROUGH
				| Options::ENABLE_TASKLISTS
				| Options::ENABLE_SMART_PUNCTUATION
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

	fn markdown_to_html(markdown: &str) -> String {
		let parser = Parser::new_ext(&markdown, Self::options());

		let approx_out_len = markdown.len() * 3 / 2;
		let mut html_output = String::with_capacity(approx_out_len);
		pulldown_cmark::html::push_html(&mut html_output, parser);
		html_output
	}


	/// returns the content of the first frontmatter block discovered,
	/// wrapped in parentheses as a requirement of the `ron` parser
	fn markdown_to_frontmatter_tokens<'a>(markdown: &'a str) -> Result<Block> {
		let frontmatter = Self::extract_frontmatter_string(markdown);
		// frontmatter
		let tokens = match frontmatter {
			Some((frontmatter, MetadataBlockKind::PlusesStyle)) => {
				let frontmatter = frontmatter.to_string();
				syn::parse_quote!({
					beet::exports::toml::from_str(#frontmatter)
				})
			}
			Some((frontmatter, MetadataBlockKind::YamlStyle)) => {
				let frontmatter = Self::yaml_frontmatter_to_ron(&frontmatter)?;
				syn::parse_quote!({
					beet::exports::ron::from_str(#frontmatter)
				})
			}
			None => {
				syn::parse_quote!({ Default::default() })
			}
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
		frontmatter.map(|f| (f, meta_block_style))
	}


	/// a custom parser for the frontmatter
	fn yaml_frontmatter_to_ron(yaml: &str) -> Result<String> {
		let lines = yaml
			.lines()
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

	pub fn parse(
		markdown: &str,
		canonical_path: CanonicalPathBuf,
		local_path: PathBuf,
	) -> Result<FuncTokens> {
		let frontmatter = Self::markdown_to_frontmatter_tokens(markdown)?;
		let html_str = Self::markdown_to_html(markdown);
		let rust_tokens = html_str
			.clone()
			.xpipe(StringToHtmlTokens::default())
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nInput: {}\nError: {}",
					html_str,
					e.to_string()
				)
			})?
			.xpipe(HtmlTokensToRust::default());

		Ok(FuncTokens {
			mod_ident: None,
			frontmatter,
			func: quote! {|| rsx! {#rust_tokens}},
			route_info: RouteInfo {
				path: RoutePath::parse_local_path(&local_path)?,
				method: Method::GET,
			},
			local_path,
			canonical_path,
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::quote;
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
# hello world
"#;


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
		expect(MarkdownToFuncTokens::markdown_to_html(MARKDOWN))
			.to_be("<h1>hello world</h1>\n");
	}

	#[test]
	fn frontmatter() {
		let frontmatter =
			MarkdownToFuncTokens::extract_frontmatter_string(MARKDOWN).unwrap();
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
	fn parse() {
		let func_tokens = MarkdownToFuncTokens::parse(
			MARKDOWN,
			CanonicalPathBuf::new_unchecked("foo"),
			"bar".into(),
		)
		.unwrap();
		expect(func_tokens.func.to_string()).to_be(
			quote! {
				rsx! {
					{
						use beet::prelude::*;
						#[allow(unused_braces)]
						RsxElement {
							tag: "h1".to_string(),
							attributes: vec![],
							children: Box::new(
								RsxText {
									value: "hello world".to_string(),
									meta: RsxNodeMeta::default(),
								}.into_node()
							),
							self_closing: false,
							meta: RsxNodeMeta {
								template_directives: vec![],
								location: None
							},
						}
						.into_node()
						.with_location(RsxMacroLocation::new(file!(), 0u32, 0u32))
					}
				}
			}
			.to_string(),
		);
	}
}
