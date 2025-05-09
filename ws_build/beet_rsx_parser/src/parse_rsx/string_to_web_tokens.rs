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
	rusty_tracker: RustyTrackerBuilder,
}

impl StringToWebTokens {
	pub fn new(source_file: WorkspacePathBuf) -> Self {
		Self {
			source_file,
			rusty_tracker: RustyTrackerBuilder::default(),
		}
	}
}

impl<T: AsRef<str>> Pipeline<T, Result<WebTokens>> for StringToWebTokens {
	fn apply(mut self, input: T) -> Result<WebTokens> {
		let (expr, remaining) = parse(input.as_ref()).map_err(|e| {
			anyhow::anyhow!("Failed to parse HTML: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}
		let mut tokens = self.rsx_parsed_expression(expr)?;
		tokens.push_directive(TemplateDirective::NodeTemplate);
		Ok(tokens)
	}
}
impl StringToWebTokens {
	fn rsx_parsed_expression(
		&mut self,
		expr: RsxParsedExpression,
	) -> Result<WebTokens> {
		if expr.len() == 1 {
			self.rsx_tokens_or_element(expr.inner().into_iter().next().unwrap())
		} else {
			WebTokens::Fragment {
				nodes: expr
					.inner()
					.into_iter()
					.map(|item| self.rsx_tokens_or_element(item))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(&self.source_file).into(),
			}
			.xok()
		}
	}

	fn rsx_tokens_or_element(
		&mut self,
		tokens: RsxTokensOrElement,
	) -> Result<WebTokens> {
		match tokens {
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
				WebTokens::Block {
					tracker: self.rusty_tracker.next_tracker(&block),
					value: block.into(),
					meta: FileSpan::new_for_file(&self.source_file).into(),
				}
				.xok()
			}
			RsxTokensOrElement::Element(el) => self.rsx_element(el),
		}
	}

	fn rsx_element(&mut self, element: RsxElement) -> Result<WebTokens> {
		match element {
			RsxElement::SelfClosing(el) => self.rsx_self_closing_element(el),
			RsxElement::Normal(el) => self.rsx_normal_element(el),
		}
	}
	fn rsx_self_closing_element(
		&mut self,
		el: RsxSelfClosingElement,
	) -> Result<WebTokens> {
		WebTokens::Element {
			component: ElementTokens {
				tag: el.0.to_string().into(),
				attributes: el
					.1
					.0
					.into_iter()
					.map(|attr| self.rsx_attribute(attr))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(&self.source_file).into(),
			},
			children: Default::default(),
			self_closing: true,
		}
		.xok()
	}

	fn rsx_normal_element(
		&mut self,
		el: RsxNormalElement,
	) -> Result<WebTokens> {
		const STRING_TAGS: [&str; 3] = ["script", "style", "code"];

		let children = if STRING_TAGS.contains(&el.0.to_string().as_str()) {
			WebTokens::Text {
				value: LitStr::new(&el.2.to_html(), Span::call_site()).into(),
				meta: FileSpan::new_for_file(&self.source_file).into(),
			}
		} else {
			self.rsx_children(el.2)?
		};

		WebTokens::Element {
			component: ElementTokens {
				tag: el.0.to_string().into(),
				attributes: el
					.1
					.0
					.into_iter()
					.map(|attr| self.rsx_attribute(attr))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(&self.source_file).into(),
			},
			// here we must descide whether to go straight into html, ie for
			// script, style and code elements
			children: Box::new(children),
			self_closing: false,
		}
		.xok()
	}

	fn rsx_children(&mut self, children: RsxChildren) -> Result<WebTokens> {
		if children.0.len() == 1 {
			self.rsx_child(children.0.into_iter().next().unwrap())
		} else {
			WebTokens::Fragment {
				nodes: children
					.0
					.into_iter()
					.map(|child| self.rsx_child(child))
					.collect::<Result<Vec<_>>>()?,
				meta: FileSpan::new_for_file(&self.source_file).into(),
			}
			.xok()
		}
	}
	fn rsx_child(&mut self, child: RsxChild) -> Result<WebTokens> {
		match child {
			RsxChild::Element(el) => self.rsx_element(el),
			RsxChild::Text(text) => self.rsx_text(text),
			RsxChild::CodeBlock(code_block) => {
				self.rsx_parsed_expression(code_block)
			}
		}
	}

	fn rsx_text(&self, text: RsxText) -> Result<WebTokens> {
		WebTokens::Text {
			value: LitStr::new(&text.0, Span::call_site()).into(),
			meta: FileSpan::new_for_file(&self.source_file).into(),
		}
		.xok()
	}

	fn rsx_attribute(
		&mut self,
		attribute: RsxAttribute,
	) -> Result<RsxAttributeTokens> {
		match attribute {
			RsxAttribute::Named(name, value)
				if let RsxAttributeValue::Default = value =>
			{
				RsxAttributeTokens::Key {
					key: name.to_string().into(),
				}
			}
			RsxAttribute::Named(name, value) => {
				match self.rsx_attribute_value(value)? {
					Expr::Lit(value) => RsxAttributeTokens::KeyValueLit {
						key: name.to_string().into(),
						value: value.lit,
					},
					value => RsxAttributeTokens::KeyValueExpr {
						tracker: self.rusty_tracker.next_tracker(&value),
						key: name.to_string().into(),
						value,
					},
				}
			}
			RsxAttribute::Spread(value) => {
				let block = self
					.rsx_parsed_expression(value)?
					.xpipe(WebTokensToRust::default());
				RsxAttributeTokens::Block {
					tracker: self.rusty_tracker.next_tracker(&block),
					block: block.into(),
				}
			}
		}
		.xok()
	}
	fn rsx_attribute_value(
		&mut self,
		value: RsxAttributeValue,
	) -> Result<Expr> {
		let expr: Expr = match value {
			RsxAttributeValue::Default => {
				unreachable!(
					"We checked for this in StringToWebTokens::rsx_attribute"
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
					self.rsx_element(value)?.xpipe(WebTokensToRust::default());
				syn::parse_quote!(#block)
			}
			RsxAttributeValue::CodeBlock(value) => {
				let block = self
					.rsx_parsed_expression(value)?
					.xpipe(WebTokensToRust::default());
				syn::parse_quote!(#block)
			}
		};
		expr.xok()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	fn str_to_ron_matcher(str: &str) -> Matcher<String> {
		str.xpipe(StringToWebTokens::default())
			.unwrap()
			.xpipe(WebTokensToTemplate::default())
			.xmap(|template| ron::ser::to_string(&template).unwrap())
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
			.to_contain("Element(tag:\"br\"");
	}
	#[test]
	#[ignore]
	fn unclosed() {
		"<div align=\"center\" />"
			.xmap(str_to_ron_matcher)
			.to_contain("Element(tag:\"br\"");
	}

	#[test]
	fn text() {
		"<div>hello</div>"
			.xmap(str_to_ron_matcher)
			.to_contain("Text(value:\"hello\"");
	}
	#[test]
	fn attributes() {
		// default
		"<br foo />"
			.xmap(str_to_ron_matcher)
			.to_contain("Key(key:\"foo\")");
		// string
		"<br foo=\"bar\"/>"
			.xmap(str_to_ron_matcher)
			.to_contain("KeyValue(key:\"foo\",value:\"bar\")");
		// bool
		"<br foo=true />"
			.xmap(str_to_ron_matcher)
			.to_contain("KeyValue(key:\"foo\",value:\"true\")");
		// number
		"<br foo=20 />"
			.xmap(str_to_ron_matcher)
			.to_contain("KeyValue(key:\"foo\",value:\"20\")");
		// ident
		"<br foo={bar} />"
			.xmap(str_to_ron_matcher)
			.to_contain("BlockValue(key:\"foo\",tracker:(index:1,");
		// element
		"<br foo={<br/>} />"
			.xmap(str_to_ron_matcher)
			.to_contain("BlockValue(key:\"foo\",tracker:(index:0,");
	}
}
