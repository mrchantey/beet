use crate::prelude::*;
use anyhow::Result;
use beet_rsx_combinator::prelude::*;
use proc_macro2::Span;
use sweet::prelude::*;
use syn::Block;
use syn::Expr;
use syn::LitStr;


/// For a given string of rsx, use [`beet_rsx_combinator`] to parse.
#[derive(Debug, Default)]
pub struct StringToHtmlTokens;

impl<T: AsRef<str>> Pipeline<T, Result<HtmlTokens>> for StringToHtmlTokens {
	fn apply(self, input: T) -> Result<HtmlTokens> {
		let (expr, remaining) = parse(input.as_ref()).map_err(|e| {
			anyhow::anyhow!("Failed to parse HTML: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}
		expr.into_html_tokens()
	}
}

trait IntoHtmlTokens {
	fn into_html_tokens(self) -> Result<HtmlTokens>;
}

impl IntoHtmlTokens for RsxParsedExpression {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		if self.len() == 1 {
			self.inner().into_iter().next().unwrap().into_html_tokens()
		} else {
			HtmlTokens::Fragment {
				nodes: self
					.inner()
					.into_iter()
					.map(IntoHtmlTokens::into_html_tokens)
					.collect::<Result<Vec<_>>>()?,
			}
			.xok()
		}
	}
}

impl IntoHtmlTokens for RsxTokensOrElement {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
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
				Ok(HtmlTokens::Block {
					value: block.into(),
				})
			}
			RsxTokensOrElement::Element(el) => el.into_html_tokens(),
		}
	}
}
impl IntoHtmlTokens for RsxElement {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		match self {
			RsxElement::SelfClosing(el) => el.into_html_tokens(),
			RsxElement::Normal(el) => el.into_html_tokens(),
		}
	}
}

impl IntoHtmlTokens for RsxSelfClosingElement {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		HtmlTokens::Element {
			component: RsxNodeTokens {
				tag: NameExpr::LitStr(
					LitStr::new(&self.0.to_string(), Span::call_site()).into(),
				),
				attributes: self
					.1
					.0
					.into_iter()
					.map(|v| v.into_rsx_attribute_tokens())
					.collect::<Result<Vec<_>>>()?,
				directives: Vec::new(),
			},
			children: Default::default(),
			self_closing: true,
		}
		.xok()
	}
}

impl IntoHtmlTokens for RsxNormalElement {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		const STRING_TAGS: [&str; 3] = ["script", "style", "code"];

		let children = if STRING_TAGS.contains(&self.0.to_string().as_str()) {
			HtmlTokens::Text {
				value: LitStr::new(&self.2.to_html(), Span::call_site()).into(),
			}
		} else {
			self.2.into_html_tokens()?
		};

		HtmlTokens::Element {
			component: RsxNodeTokens {
				tag: NameExpr::LitStr(
					LitStr::new(&self.0.to_string(), Span::call_site()).into(),
				),
				attributes: self
					.1
					.0
					.into_iter()
					.map(|v| v.into_rsx_attribute_tokens())
					.collect::<Result<Vec<_>>>()?,
				directives: Vec::new(),
			},
			// here we must descide whether to go straight into html, ie for
			// script, style and code elements
			children: Box::new(children),
			self_closing: false,
		}
		.xok()
	}
}

impl IntoHtmlTokens for RsxChildren {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		if self.0.len() == 1 {
			self.0
				.into_iter()
				.next()
				.unwrap()
				.xmap(IntoHtmlTokens::into_html_tokens)
		} else {
			HtmlTokens::Fragment {
				nodes: self
					.0
					.into_iter()
					.map(IntoHtmlTokens::into_html_tokens)
					.collect::<Result<_>>()?,
			}
			.xok()
		}
	}
}

impl IntoHtmlTokens for RsxChild {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		match self {
			RsxChild::Element(val) => val.into_html_tokens(),
			RsxChild::Text(val) => val.into_html_tokens(),
			RsxChild::CodeBlock(val) => val.into_html_tokens(),
		}
	}
}


impl IntoHtmlTokens for RsxText {
	fn into_html_tokens(self) -> Result<HtmlTokens> {
		HtmlTokens::Text {
			value: LitStr::new(&self.0, Span::call_site()).into(),
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
				value: value.try_into()?,
			}
			.xok(),
			RsxAttribute::Spread(value) => {
				let block = value
					.into_html_tokens()?
					.xpipe(HtmlTokensToRust::new_no_location());
				RsxAttributeTokens::Block {
					block: block.into(),
				}
				.xok()
			}
		}
	}
}

impl TryInto<Spanner<Expr>> for RsxAttributeValue {
	type Error = anyhow::Error;
	fn try_into(self) -> Result<Spanner<Expr>, Self::Error> {
		let expr: Expr = match self {
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
					.into_html_tokens()?
					.xpipe(HtmlTokensToRust::new_no_location());
				syn::parse_quote!(#block)
			}
			RsxAttributeValue::CodeBlock(value) => {
				let block = value
					.into_html_tokens()?
					.xpipe(HtmlTokensToRust::new_no_location());
				syn::parse_quote!(#block)
			}
		};
		expr.xinto::<Spanner<Expr>>().xok()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	fn str_to_ron_matcher(str: &str) -> Matcher<String> {
		str.xpipe(StringToHtmlTokens)
			.unwrap()
			.xpipe(HtmlTokensToRon::new_no_location())
			.to_string()
			.xpect()
	}
	// fn str_to_rust_matcher(str: &str) -> Matcher<String> {
	// 	str.xpipe(StringToHtmlTokens)
	// 		.unwrap()
	// 		.xpipe(HtmlTokensToRust::default())
	// 		.to_token_stream()
	// 		.to_string()
	// 		.xpect()
	// }

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
