use crate::prelude::*;
use anyhow::Result;
use beet_jsx::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use sweet::prelude::*;
use syn::Block;
use syn::Expr;
use syn::LitStr;

#[derive(Debug, Default)]
pub struct StringToHtmlTokens;

impl Pipeline<String, Result<HtmlTokens>> for StringToHtmlTokens {
	fn apply(self, input: String) -> Result<HtmlTokens> {
		let (el, remaining) = parse(&input).map_err(|e| {
			anyhow::anyhow!("Failed to parse HTML: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}
		Ok(el.into())
	}
}

impl Into<HtmlTokens> for RsxParsedExpression {
	fn into(self) -> HtmlTokens {
		// TODO this is a hack, we need to be able to handle this
		// expression element placeholder technique all the way
		// through the pipeline.
		// currently its either expression or fragment
		if self.elements.is_empty() {
			// panic!("Expression: {}", self.tokens);
			let block: Block = syn::parse_str(&format!("{{{}}}", &self.tokens))
				.map_err(|e| {
					anyhow::anyhow!(
						"Failed to parse block:\nblock:{}\nerror:{}",
						self.tokens,
						e.to_string()
					)
				})
				.unwrap();
			HtmlTokens::Block {
				value: block.into(),
			}
		} else if self.elements.len() == 1 {
			self.elements.into_iter().next().unwrap().1.into()
		} else {
			HtmlTokens::Fragment {
				nodes: self
					.elements
					.into_iter()
					.map(|(_, e)| e.into())
					.collect(),
			}
		}
	}
}

impl Into<HtmlTokens> for RsxElement {
	fn into(self) -> HtmlTokens {
		match self {
			RsxElement::SelfClosing(el) => el.into(),
			RsxElement::Normal(el) => el.into(),
		}
	}
}

impl Into<HtmlTokens> for RsxSelfClosingElement {
	fn into(self) -> HtmlTokens {
		HtmlTokens::Element {
			component: RsxNodeTokens {
				tag: NameExpr::LitStr(
					LitStr::new(&self.0.to_string(), Span::call_site()).into(),
				),
				tokens: TokenStream::new(),
				attributes: self.1.0.into_iter().map(|v| v.into()).collect(),
				directives: Vec::new(),
			},
			children: Default::default(),
			self_closing: true,
		}
	}
}

impl Into<HtmlTokens> for RsxNormalElement {
	fn into(self) -> HtmlTokens {
		const STRING_TAGS: [&str; 3] = ["script", "style", "code"];

		let children = if STRING_TAGS.contains(&self.0.to_string().as_str()) {
			HtmlTokens::Text {
				value: LitStr::new(&self.2.to_html(), Span::call_site()).into(),
			}
		} else {
			self.2.into()
		};

		HtmlTokens::Element {
			component: RsxNodeTokens {
				tag: NameExpr::LitStr(
					LitStr::new(&self.0.to_string(), Span::call_site()).into(),
				),
				tokens: TokenStream::new(),
				attributes: self.1.0.into_iter().map(|v| v.into()).collect(),
				directives: Vec::new(),
			},
			// here we must descide whether to go straight into html, ie for
			// script, style and code elements
			children: Box::new(children),
			self_closing: false,
		}
	}
}


impl Into<HtmlTokens> for RsxChildren {
	fn into(self) -> HtmlTokens {
		let children: Vec<HtmlTokens> =
			self.0.into_iter().map(|v| v.into()).collect();
		if children.len() == 1 {
			children.into_iter().next().unwrap()
		} else {
			HtmlTokens::Fragment { nodes: children }
		}
	}
}

impl Into<HtmlTokens> for RsxChild {
	fn into(self) -> HtmlTokens {
		match self {
			RsxChild::Element(val) => val.into(),
			RsxChild::Text(val) => val.into(),
			RsxChild::CodeBlock(val) => val.into(),
		}
	}
}

impl Into<HtmlTokens> for RsxText {
	fn into(self) -> HtmlTokens {
		HtmlTokens::Text {
			value: LitStr::new(&self.0, Span::call_site()).into(),
		}
	}
}

impl Into<RsxAttributeTokens> for RsxAttribute {
	fn into(self) -> RsxAttributeTokens {
		match self {
			RsxAttribute::Named(name, value)
				if let RsxAttributeValue::Default = value =>
			{
				RsxAttributeTokens::Key {
					key: name.to_string().into(),
				}
			}
			RsxAttribute::Named(name, value) => RsxAttributeTokens::KeyValue {
				key: name.to_string().into(),
				value: value.into(),
			},
			RsxAttribute::Spread(_value) => {
				todo!()
			}
		}
	}
}

impl Into<Spanner<Expr>> for RsxAttributeValue {
	fn into(self) -> Spanner<Expr> {
		let expr: Expr = match self {
			RsxAttributeValue::Default => {
				panic!(
					"this would be RsxAttributeTokens::Key, please check before calling this"
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
			RsxAttributeValue::Element(_val) => todo!(
				"
			same with this one, maybe need to extend RsxAttributeTokens to support this"
			),
			RsxAttributeValue::CodeBlock(_val) => todo!(
				"not sure how this will work, its too early to parse into tokens"
			),
		};
		expr.into()
	}
}
