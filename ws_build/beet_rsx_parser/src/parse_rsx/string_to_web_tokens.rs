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
	span: Option<FileSpan>,
}

impl StringToWebTokens {
	pub fn new(span: Option<FileSpan>) -> Self { Self { span } }
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
		let mut tokens = expr.into_web_tokens()?;
		if let Some(span) = self.span {
			tokens.meta_mut().location = Some(span);
		}
		Ok(tokens)
	}
}

trait IntoWebTokens {
	fn into_web_tokens(self) -> Result<WebTokens>;
}

impl IntoWebTokens for RsxParsedExpression {
	fn into_web_tokens(self) -> Result<WebTokens> {
		if self.len() == 1 {
			self.inner().into_iter().next().unwrap().into_web_tokens()
		} else {
			WebTokens::Fragment {
				nodes: self
					.inner()
					.into_iter()
					.map(IntoWebTokens::into_web_tokens)
					.collect::<Result<Vec<_>>>()?,
				meta: Default::default(),
			}
			.xok()
		}
	}
}

impl IntoWebTokens for RsxTokensOrElement {
	fn into_web_tokens(self) -> Result<WebTokens> {
		match self {
			RsxTokensOrElement::Tokens(tokens) => {
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
					meta: Default::default(),
				})
			}
			RsxTokensOrElement::Element(el) => el.into_web_tokens(),
		}
	}
}
impl IntoWebTokens for RsxElement {
	fn into_web_tokens(self) -> Result<WebTokens> {
		match self {
			RsxElement::SelfClosing(el) => el.into_web_tokens(),
			RsxElement::Normal(el) => el.into_web_tokens(),
		}
	}
}

impl IntoWebTokens for RsxSelfClosingElement {
	fn into_web_tokens(self) -> Result<WebTokens> {
		WebTokens::Element {
			component: ElementTokens {
				tag: self.0.to_string().into(),
				attributes: self
					.1
					.0
					.into_iter()
					.map(|v| v.into_rsx_attribute_tokens())
					.collect::<Result<Vec<_>>>()?,
				meta: Default::default(),
			},
			children: Default::default(),
			self_closing: true,
		}
		.xok()
	}
}

impl IntoWebTokens for RsxNormalElement {
	fn into_web_tokens(self) -> Result<WebTokens> {
		const STRING_TAGS: [&str; 3] = ["script", "style", "code"];

		let children = if STRING_TAGS.contains(&self.0.to_string().as_str()) {
			WebTokens::Text {
				value: LitStr::new(&self.2.to_html(), Span::call_site()).into(),
				meta: Default::default(),
			}
		} else {
			self.2.into_web_tokens()?
		};

		WebTokens::Element {
			component: ElementTokens {
				tag: self.0.to_string().into(),
				attributes: self
					.1
					.0
					.into_iter()
					.map(|v| v.into_rsx_attribute_tokens())
					.collect::<Result<Vec<_>>>()?,
				meta: Default::default(),
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
	fn into_web_tokens(self) -> Result<WebTokens> {
		if self.0.len() == 1 {
			self.0
				.into_iter()
				.next()
				.unwrap()
				.xmap(IntoWebTokens::into_web_tokens)
		} else {
			WebTokens::Fragment {
				nodes: self
					.0
					.into_iter()
					.map(IntoWebTokens::into_web_tokens)
					.collect::<Result<_>>()?,
				meta: Default::default(),
			}
			.xok()
		}
	}
}

impl IntoWebTokens for RsxChild {
	fn into_web_tokens(self) -> Result<WebTokens> {
		match self {
			RsxChild::Element(val) => val.into_web_tokens(),
			RsxChild::Text(val) => val.into_web_tokens(),
			RsxChild::CodeBlock(val) => val.into_web_tokens(),
		}
	}
}


impl IntoWebTokens for RsxText {
	fn into_web_tokens(self) -> Result<WebTokens> {
		WebTokens::Text {
			value: LitStr::new(&self.0, Span::call_site()).into(),
			meta: Default::default(),
		}
		.xok()
	}
}


trait IntoRsxAttributeTokens {
	fn into_rsx_attribute_tokens(self) -> Result<RsxAttributeTokens>;
}

impl IntoRsxAttributeTokens for RsxAttribute {
	fn into_rsx_attribute_tokens(self) -> Result<RsxAttributeTokens> {
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
				value: attribute_value_to_expr(value)?,
			}
			.xok(),
			RsxAttribute::Spread(value) => {
				let block =
					value.into_web_tokens()?.xpipe(WebTokensToRust::default());
				RsxAttributeTokens::Block {
					block: block.into(),
				}
				.xok()
			}
		}
	}
}

fn attribute_value_to_expr(value: RsxAttributeValue) -> Result<Expr> {
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
			let block =
				value.into_web_tokens()?.xpipe(WebTokensToRust::default());
			syn::parse_quote!(#block)
		}
		RsxAttributeValue::CodeBlock(value) => {
			let block =
				value.into_web_tokens()?.xpipe(WebTokensToRust::default());
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
