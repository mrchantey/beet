use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx_combinator::prelude::*;
use proc_macro2::Span;
use sweet::prelude::*;
use syn::Block;
use syn::Expr;
use syn::LitStr;

/// For a given string of rsx, use [`beet_rsx_combinator`] to parse.
#[derive(Debug, Default)]
pub struct StringToWebTokens {
	source_file: WorkspacePathBuf,
}

impl StringToWebTokens {
	pub fn new(source_file: WorkspacePathBuf) -> Self { Self { source_file } }
}

impl<T: AsRef<str>> Pipeline<T, Result<WebTokens>> for StringToWebTokens {
	fn apply(self, input: T) -> Result<WebTokens> {
		let (expr, remaining) = parse(input.as_ref()).map_err(|e| {
			anyhow::anyhow!("Failed to parse HTML: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}
		let mut tokens = expr.into_web_tokens(&self.source_file)?;
		tokens.push_directive(TemplateDirective::RsxTemplate);
		Ok(tokens)
	}
}

trait IntoWebTokens {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens>;
}

impl IntoWebTokens for RsxParsedExpression {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		if self.len() == 1 {
			self.inner()
				.into_iter()
				.next()
				.unwrap()
				.into_web_tokens(source_file)
		} else {
			WebTokens::Fragment {
				nodes: self
					.inner()
					.into_iter()
					.map(|item| item.into_web_tokens(source_file))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(source_file).into(),
			}
			.xok()
		}
	}
}

impl IntoWebTokens for RsxTokensOrElement {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		match self {
			RsxTokensOrElement::Tokens(tokens) => {
				// TODO this is incorrect, what we need is a new type,
				// like a PartialBlock or something that allows for interspersed
				// tokens and elements
				// TODO this is incorrect, what we need is a new type,
				// like a PartialBlock or something that allows for interspersed
				// tokens and elements
				let block: Block = syn::parse_str(&format!("{{{}}}", &tokens))
					.map_err(|e| {
						anyhow::anyhow!(
							"\nWarning: This parser is a wip so the error may not be accurate \n\
									Failed to parse block:\nblock:{}\nerror:{}",
							tokens,
							e.to_string()
						)
					})?;
				Ok(WebTokens::Block {
					value: block.into(),
					meta: FileSpan::new_for_file(source_file).into(),
				})
			}
			RsxTokensOrElement::Element(el) => el.into_web_tokens(source_file),
		}
	}
}

impl IntoWebTokens for RsxElement {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		match self {
			RsxElement::SelfClosing(el) => el.into_web_tokens(source_file),
			RsxElement::Normal(el) => el.into_web_tokens(source_file),
		}
	}
}

impl IntoWebTokens for RsxSelfClosingElement {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		WebTokens::Element {
			component: ElementTokens {
				tag: self.0.to_string().into(),
				attributes: self
					.1
					.0
					.into_iter()
					.map(|v| v.into_rsx_attribute_tokens(source_file))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(source_file).into(),
			},
			children: Default::default(),
			self_closing: true,
		}
		.xok()
	}
}

impl IntoWebTokens for RsxNormalElement {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		const STRING_TAGS: [&str; 3] = ["script", "style", "code"];

		let children = if STRING_TAGS.contains(&self.0.to_string().as_str()) {
			WebTokens::Text {
				value: LitStr::new(&self.2.to_html(), Span::call_site()).into(),
				meta: FileSpan::new_for_file(source_file).into(),
			}
		} else {
			self.2.into_web_tokens(source_file)?
		};

		WebTokens::Element {
			component: ElementTokens {
				tag: self.0.to_string().into(),
				attributes: self
					.1
					.0
					.into_iter()
					.map(|v| v.into_rsx_attribute_tokens(source_file))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(source_file).into(),
			},
			// here we must descide whether to go straight into html, ie for
			// script, style and code elements
			children: Box::new(children),
			self_closing: false,
		}
		.xok()
	}
}

impl IntoWebTokens for RsxChildren {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		if self.0.len() == 1 {
			self.0
				.into_iter()
				.next()
				.unwrap()
				.into_web_tokens(source_file)
		} else {
			WebTokens::Fragment {
				nodes: self
					.0
					.into_iter()
					.map(|item| item.into_web_tokens(source_file))
					.collect::<Result<_>>()?,
				meta: FileSpan::new_for_file(source_file).into(),
			}
			.xok()
		}
	}
}

impl IntoWebTokens for RsxChild {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		match self {
			RsxChild::Element(val) => val.into_web_tokens(source_file),
			RsxChild::Text(val) => val.into_web_tokens(source_file),
			RsxChild::CodeBlock(val) => val.into_web_tokens(source_file),
		}
	}
}

impl IntoWebTokens for RsxText {
	fn into_web_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<WebTokens> {
		WebTokens::Text {
			value: LitStr::new(&self.0, Span::call_site()).into(),
			meta: FileSpan::new_for_file(source_file).into(),
		}
		.xok()
	}
}

trait IntoRsxAttributeTokens {
	fn into_rsx_attribute_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<RsxAttributeTokens>;
}

impl IntoRsxAttributeTokens for RsxAttribute {
	fn into_rsx_attribute_tokens(
		self,
		source_file: &WorkspacePathBuf,
	) -> Result<RsxAttributeTokens> {
		match self {
			RsxAttribute::Named(name, value)
				if let RsxAttributeValue::Default = value =>
			{
				RsxAttributeTokens::Key {
					key: name.to_string().into(),
				}
				.xok()
			}
			RsxAttribute::Named(name, value) => RsxAttributeTokens::KeyValue {
				key: name.to_string().into(),
				value: attribute_value_to_expr(value, source_file)?,
			}
			.xok(),
			RsxAttribute::Spread(value) => {
				let block = value
					.into_web_tokens(source_file)?
					.xpipe(WebTokensToRust::default());
				RsxAttributeTokens::Block {
					block: block.into(),
				}
				.xok()
			}
		}
	}
}

fn attribute_value_to_expr(
	value: RsxAttributeValue,
	source_file: &WorkspacePathBuf,
) -> Result<Expr> {
	let expr: Expr = match value {
		RsxAttributeValue::Default => {
			unreachable!(
				"We checked for this in RsxAttribute::into_rsx_attribute_tokens"
			)
		}
		RsxAttributeValue::Boolean(val) => {
			let val = val.0;
			syn::parse_quote!(#val)
		}
		RsxAttributeValue::Number(val) => {
			let val = val.0;
			syn::parse_quote!(#val)
		}
		RsxAttributeValue::Str(val) => {
			let val = val.to_string();
			syn::parse_quote!(#val)
		}
		RsxAttributeValue::Element(value) => {
			let block = value
				.into_web_tokens(source_file)?
				.xpipe(WebTokensToRust::default());
			syn::parse_quote!(#block)
		}
		RsxAttributeValue::CodeBlock(value) => {
			let block = value
				.into_web_tokens(source_file)?
				.xpipe(WebTokensToRust::default());
			syn::parse_quote!(#block)
		}
	};
	expr.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	fn str_to_ron_matcher(str: &str) -> Matcher<String> {
		str.xpipe(StringToWebTokens::default())
			.unwrap()
			.xpipe(WebTokensToRon::default())
			.to_string()
			.xpect()
	}

	#[allow(unused)]
	fn str_to_rust_matcher(str: &str) -> Matcher<String> {
		str.xpipe(StringToWebTokens::default())
			.unwrap()
			.xpipe(WebTokensToRust::default())
			.to_token_stream()
			.to_string()
			.xpect()
	}

	#[test]
	fn element() {
		"<br/>"
			.xmap(str_to_ron_matcher)
			.to_contain("Element (tag : \"br\"");
	}
	#[test]
	#[ignore]
	fn unclosed() {
		"<div align=\"center\" />"
			.xmap(str_to_ron_matcher)
			.to_contain("Element (tag : \"br\"");
	}

	#[test]
	fn text() {
		"<div>hello</div>"
			.xmap(str_to_ron_matcher)
			.to_contain("Text (value : \"hello\"");
	}
	#[test]
	fn attributes() {
		// default
		"<br foo />"
			.xmap(str_to_ron_matcher)
			.to_contain("Key (key : \"foo\")");
		// string
		"<br foo=\"bar\"/>"
			.xmap(str_to_ron_matcher)
			.to_contain("KeyValue (key : \"foo\" , value : \"bar\")");
		// bool
		"<br foo=true />"
			.xmap(str_to_ron_matcher)
			.to_contain("KeyValue (key : \"foo\" , value : \"true\")");
		// number
		"<br foo=20 />"
			.xmap(str_to_ron_matcher)
			.to_contain("KeyValue (key : \"foo\" , value : \"20\")");
		// ident
		"<br foo={bar} />"
			.xmap(str_to_ron_matcher)
			.to_contain("BlockValue (key : \"foo\" , tracker : RustyTracker");
		// element
		"<br foo={<br/>} />"
			.xmap(str_to_ron_matcher)
			.to_contain("BlockValue (key : \"foo\" , tracker : RustyTracker");
	}
}
